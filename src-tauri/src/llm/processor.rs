use super::client::LlmClient;
use super::embed::Embedder;
use super::prompts;
use super::rag;
use crate::db::Database;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize, Debug)]
struct StructuredConversation {
    title: String,
    overview: String,
    emoji: String,
    category: String,
}

#[derive(Deserialize, Debug)]
struct ActionItem {
    description: String,
    priority: String,
    confidence: f64,
}

#[derive(Deserialize, Debug)]
struct Memory {
    content: String,
    category: String,
}

/// Process a completed conversation: extract structure, action items, and memories
pub async fn process_conversation(
    client: &LlmClient,
    db: &Arc<Database>,
    conversation_id: &str,
    transcript: &str,
) -> Result<(), String> {
    if transcript.trim().len() < 20 {
        log::info!("Transcript too short to process, skipping");
        return Ok(());
    }

    log::info!(
        "Processing conversation {} ({} chars)",
        conversation_id,
        transcript.len()
    );

    let embedder = Embedder::new();
    let mut conv_overview_for_embed: Option<String> = None;
    let mut conv_title_for_embed: Option<String> = None;

    // Extract structure
    match extract_structure(client, transcript).await {
        Ok(structured) => {
            let conn = db.conn();
            let _ = conn.execute(
                "UPDATE conversations SET title = ?1, overview = ?2, emoji = ?3, category = ?4, status = 'completed'
                 WHERE id = ?5",
                rusqlite::params![
                    structured.title,
                    structured.overview,
                    structured.emoji,
                    structured.category,
                    conversation_id,
                ],
            );
            log::info!("Structure: {} {}", structured.emoji, structured.title);
            conv_title_for_embed = Some(structured.title.clone());
            conv_overview_for_embed = Some(structured.overview.clone());
        }
        Err(e) => log::error!("Structure extraction failed: {}", e),
    }

    // Extract action items
    match extract_action_items(client, transcript).await {
        Ok(items) => {
            let conn = db.conn();
            for item in &items {
                if item.confidence < 0.7 {
                    continue;
                }
                let id = uuid::Uuid::new_v4().to_string();
                let _ = conn.execute(
                    "INSERT INTO action_items (id, description, priority, confidence, conversation_id)
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                    rusqlite::params![
                        id,
                        item.description,
                        item.priority,
                        item.confidence,
                        conversation_id,
                    ],
                );
            }
            log::info!("Extracted {} action items", items.len());
        }
        Err(e) => log::error!("Action item extraction failed: {}", e),
    }

    // Extract memories — route each through the dedup pipeline (same path
    // chat tool uses) so processor-extracted memories don't duplicate ones
    // the user already created via chat, and vice versa. Embedding is done
    // inside dedup_or_create_memory so we don't need a second pass here.
    match extract_memories(client, db, transcript).await {
        Ok(memories) => {
            let n = memories.len();
            for mem in &memories {
                if let Err(e) = super::tools::dedup_or_create_memory(
                    &mem.content,
                    &mem.category,
                    Some(conversation_id),
                    client,
                    db,
                    &embedder,
                )
                .await
                {
                    log::warn!("processor: dedup_or_create_memory failed: {}", e);
                }
            }
            log::info!("Extracted {} memories (after dedup)", n);
        }
        Err(e) => log::error!("Memory extraction failed: {}", e),
    }

    // Generate embedding for the conversation overview (separate from memories,
    // which were embedded inside dedup_or_create_memory above).
    if let (Some(title), Some(overview)) = (conv_title_for_embed, conv_overview_for_embed) {
        let combined = format!("{}\n{}", title, overview);
        if let Err(e) =
            rag::store_embedding(&embedder, db, "conversation", conversation_id, &combined).await
        {
            log::warn!("Failed to embed conversation overview: {}", e);
        }
    }

    Ok(())
}

async fn extract_structure(
    client: &LlmClient,
    transcript: &str,
) -> Result<StructuredConversation, String> {
    let response = client.chat(prompts::STRUCTURE_PROMPT, transcript).await?;

    let cleaned = clean_json(&response);
    serde_json::from_str::<StructuredConversation>(&cleaned).map_err(|e| {
        format!(
            "Failed to parse structure JSON: {} — response: {}",
            e, response
        )
    })
}

async fn extract_action_items(
    client: &LlmClient,
    transcript: &str,
) -> Result<Vec<ActionItem>, String> {
    let response = client
        .chat(prompts::ACTION_ITEMS_PROMPT, transcript)
        .await?;

    let cleaned = clean_json(&response);
    serde_json::from_str::<Vec<ActionItem>>(&cleaned).map_err(|e| {
        format!(
            "Failed to parse action items JSON: {} — response: {}",
            e, response
        )
    })
}

async fn extract_memories(
    client: &LlmClient,
    db: &Arc<Database>,
    transcript: &str,
) -> Result<Vec<Memory>, String> {
    // Inject the user's recent memories so the model can avoid recreating
    // duplicates. Cap at ~50 to keep the prompt bounded.
    let existing = {
        let conn = db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT category, content FROM memories
                 WHERE is_dismissed = 0
                 ORDER BY created_at DESC LIMIT 50",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        if rows.is_empty() {
            "(none yet)".to_string()
        } else {
            rows.iter()
                .map(|(cat, c)| format!("- [{}] {}", cat, c))
                .collect::<Vec<_>>()
                .join("\n")
        }
    };

    let prompt = prompts::MEMORIES_PROMPT.replace("{existing_memories}", &existing);
    let response = client.chat(&prompt, transcript).await?;

    let cleaned = clean_json(&response);
    serde_json::from_str::<Vec<Memory>>(&cleaned).map_err(|e| {
        format!(
            "Failed to parse memories JSON: {} — response: {}",
            e, response
        )
    })
}

/// Strip markdown code fences from LLM response
fn clean_json(s: &str) -> String {
    let s = s.trim();
    let s = s.strip_prefix("```json").unwrap_or(s);
    let s = s.strip_prefix("```").unwrap_or(s);
    let s = s.strip_suffix("```").unwrap_or(s);
    s.trim().to_string()
}
