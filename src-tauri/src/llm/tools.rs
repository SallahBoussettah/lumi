use super::client::{LlmClient, ToolDef, ToolFunction};
use super::embed::Embedder;
use super::prompts;
use super::rag;
use crate::db::Database;
use serde_json::{json, Value};
use std::sync::Arc;

/// Returns the tool definitions exposed to the LLM during chat.
pub fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "create_task".into(),
                description: "Create a new task (action item) for the user. Use this when the user explicitly asks you to add, create, or remember a task.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "description": {
                            "type": "string",
                            "description": "Short description of the task, starts with a verb (e.g. 'Read The Overstory', 'Email Marcus about Q4 plan')"
                        },
                        "priority": {
                            "type": "string",
                            "enum": ["high", "medium", "low"],
                            "description": "Task priority. Use 'high' for urgent or due-today items, 'medium' for this-week, 'low' for no specific deadline."
                        },
                        "due_at": {
                            "type": "string",
                            "description": "Optional due date as 'YYYY-MM-DD' (date-only) — copy the EXACT ISO value from the DATE LOOKUP table in the system prompt for the day the user mentioned. Do NOT compute the date yourself. Omit entirely if the user gave no deadline."
                        }
                    },
                    "required": ["description", "priority"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "complete_task".into(),
                description: "Mark a task as completed. Search by keywords from the task description.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "search": {
                            "type": "string",
                            "description": "Keywords to find the task by description (case-insensitive substring match)"
                        }
                    },
                    "required": ["search"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "update_task".into(),
                description: "Update an existing task's description, priority, or due date. Search to find the task first.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "search": {
                            "type": "string",
                            "description": "Keywords to find the task to update"
                        },
                        "new_description": { "type": "string" },
                        "new_priority": { "type": "string", "enum": ["high", "medium", "low"] },
                        "new_due_at": { "type": "string", "description": "ISO 8601 timestamp" }
                    },
                    "required": ["search"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "list_tasks".into(),
                description: "List the user's tasks. Use this when the user asks what's on their plate, what tasks they have, etc.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "filter": {
                            "type": "string",
                            "enum": ["all", "pending", "completed"],
                            "description": "Which tasks to show. Defaults to 'pending'."
                        }
                    }
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "create_memory".into(),
                description: "Save a fact or piece of knowledge for future reference. Use only when the user explicitly asks you to remember something.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "content": {
                            "type": "string",
                            "description": "The fact, ≤25 words, specific and concrete. PRESERVE the user's exact wording for proper nouns (names, places, project names) and for currency/unit terms (e.g. if they say 'dirhams' write 'dirhams', NOT 'AED' or 'MAD'; if they say 'bucks' write 'bucks', NOT 'USD'). Don't 'normalize' or 'translate' user vocabulary into codes or abbreviations."
                        },
                        "category": {
                            "type": "string",
                            "enum": ["system", "interesting"],
                            "description": "'system' = fact about the user. 'interesting' = external knowledge or recommendation."
                        }
                    },
                    "required": ["content", "category"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "update_memory".into(),
                description: "Edit an existing memory's content. Find by keywords. \
                    CRITICAL: `new_content` REPLACES the entire memory text — it is NOT a patch \
                    or delta. You MUST include everything from the original that should remain, \
                    PLUS the user's new information, in one self-contained sentence. \
                    Example: original = 'works as a full stack developer using Tauri and React'. \
                    User says 'I also use Rust on the backend'. \
                    new_content MUST be 'works as a full stack developer using Tauri (Rust \
                    backend) and React' — NOT 'uses Rust' or 'uses Rust and React' (those \
                    DELETE the Tauri/full-stack info). When in doubt, write the new memory as \
                    if you were composing it from scratch, including all still-true context.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "search": {
                            "type": "string",
                            "description": "Keywords from the existing memory to find it (case-insensitive substring)."
                        },
                        "new_content": {
                            "type": "string",
                            "description": "The COMPLETE replacement memory text — must preserve all still-relevant info from the original plus the user's update. Aim for one full sentence (up to 25 words), not a fragment."
                        }
                    },
                    "required": ["search", "new_content"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "delete_memory".into(),
                description: "Remove a memory permanently. Use when the user says it's wrong, irrelevant, or asks to forget it.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "search": {
                            "type": "string",
                            "description": "Keywords from the memory to find and delete."
                        }
                    },
                    "required": ["search"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "list_memories".into(),
                description: "List the user's saved memories. Useful when the user asks 'what do you remember about X', 'show me my memories', or wants an audit.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "search": {
                            "type": "string",
                            "description": "Optional keyword filter. If omitted, returns recent memories."
                        }
                    }
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "search_memories".into(),
                description: "Semantic search across the user's memories. Use when the auto-retrieved \
                    context above doesn't contain what you need to answer — e.g. user asks about a \
                    specific person, project, or topic and you want to find every related memory. \
                    Returns the top matches with relevance scores.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Free-text query. Searches by meaning, not exact keywords."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results. Default 5, max 20."
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "search_conversations".into(),
                description: "Semantic search across captured conversations (titles + overviews). \
                    Use when the user asks about past discussions, meetings, or what was said about \
                    a topic. Returns top matches with snippets.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Free-text query."
                        },
                        "limit": {
                            "type": "integer",
                            "description": "Max results. Default 5, max 10."
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "get_today_summary".into(),
                description: "Returns a quick digest of today's activity: new conversations \
                    captured, new memories extracted, pending and completed tasks. Use when the \
                    user asks 'what happened today', 'what did I do', or wants a daily snapshot.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        },
        ToolDef {
            kind: "function".into(),
            function: ToolFunction {
                name: "end_voice_session".into(),
                description: "Closes the voice conversation overlay. Only call this when the USER \
                    explicitly signals they are done with the entire conversation, not just one \
                    answer. Recognized wrap-up signals (English): 'thanks, that's all', 'thank \
                    you, that's everything', 'I'm done', 'that's it for now', 'we're good', 'all \
                    good', 'nothing else', 'goodbye', 'bye', 'talk later', 'see you', 'cool, \
                    we're good', 'I'm out'. Also non-English equivalents like 'merci c'est tout' \
                    or 'shukran khlas'. STRICT RULES: Do NOT call this after fulfilling a request \
                    — the user may have more. Do NOT call it on first turns or any message that \
                    contains a question, instruction, or new request. Do NOT add 'talk soon' or \
                    'bye' to your text reply unless you are also calling this tool. When you DO \
                    call it, pair with one short warm farewell sentence ('Talk soon.', 'Anytime, \
                    later.') as your text reply.".into(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "farewell": {
                            "type": "string",
                            "description": "Optional brief one-liner farewell, e.g. 'Talk to you soon.' This is for the tool log only — your actual spoken farewell should be in your normal text response."
                        }
                    }
                }),
            },
        },
    ]
}

/// Execute a tool call against the database. Returns a string that gets fed
/// back to the model as the tool result.
pub async fn execute_tool(
    name: &str,
    arguments: &str,
    llm: &LlmClient,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<String, String> {
    let args: Value = serde_json::from_str(arguments)
        .map_err(|e| format!("Invalid tool arguments JSON: {} (raw: {})", e, arguments))?;

    match name {
        "create_task" => create_task(&args, db).await,
        "complete_task" => complete_task(&args, db).await,
        "update_task" => update_task(&args, db).await,
        "list_tasks" => list_tasks(&args, db).await,
        "create_memory" => create_memory(&args, llm, db, embedder).await,
        "update_memory" => update_memory_tool(&args, db, embedder).await,
        "delete_memory" => delete_memory_tool(&args, db).await,
        "list_memories" => list_memories_tool(&args, db).await,
        "search_memories" => search_memories_tool(&args, db, embedder).await,
        "search_conversations" => search_conversations_tool(&args, db, embedder).await,
        "get_today_summary" => get_today_summary_tool(db).await,
        "end_voice_session" => {
            // Pure signal — the frontend reads the tool name out of the
            // turn result and closes voice mode after the farewell finishes.
            let farewell = args
                .get("farewell")
                .and_then(Value::as_str)
                .unwrap_or("Closing voice mode.");
            Ok(format!("OK ({})", farewell))
        }
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

async fn create_task(args: &Value, db: &Arc<Database>) -> Result<String, String> {
    let description = args
        .get("description")
        .and_then(Value::as_str)
        .ok_or("Missing 'description'")?;
    let priority = args
        .get("priority")
        .and_then(Value::as_str)
        .unwrap_or("medium");
    let due_at = args.get("due_at").and_then(Value::as_str);

    // Dedup: if a pending task already has near-identical description, skip
    let pattern = format!("%{}%", description.to_lowercase());
    let existing: Option<String> = {
        let conn = db.conn();
        conn.query_row(
            "SELECT description FROM action_items
             WHERE completed = 0 AND LOWER(description) LIKE ?1 LIMIT 1",
            [&pattern],
            |row| row.get::<_, String>(0),
        )
        .ok()
    };
    if let Some(existing_desc) = existing {
        return Ok(format!(
            "A similar task already exists: \"{}\" (no duplicate created)",
            existing_desc
        ));
    }

    let id = uuid::Uuid::new_v4().to_string();
    let conn = db.conn();
    conn.execute(
        "INSERT INTO action_items (id, description, priority, due_at, confidence)
         VALUES (?1, ?2, ?3, ?4, 1.0)",
        rusqlite::params![id, description, priority, due_at],
    )
    .map_err(|e| format!("DB insert failed: {}", e))?;

    Ok(format!(
        "Task created: \"{}\" [{}{}]",
        description,
        priority,
        due_at.map(|d| format!(", due {}", d)).unwrap_or_default()
    ))
}

async fn complete_task(args: &Value, db: &Arc<Database>) -> Result<String, String> {
    let search = args
        .get("search")
        .and_then(Value::as_str)
        .ok_or("Missing 'search'")?;

    // Fetch all pending tasks once, then score by how many search words appear
    // in the description. A strict LIKE %search% match misses cases where the
    // model phrases the search differently from the task ("voice mode rebuild"
    // vs the task "Commit voice mode rebuild before launch") — multi-word AND
    // matching is much more robust.
    let pending: Vec<(String, String)> = {
        let conn = db.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, description FROM action_items
                 WHERE completed = 0
                 ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<(String, String)> = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    if pending.is_empty() {
        return Ok("No pending tasks to complete.".to_string());
    }

    let words: Vec<String> = search
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2) // drop noise words like "to", "a", "is"
        .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()).to_string())
        .filter(|w| !w.is_empty())
        .collect();

    // Score each task by how many search words appear in its description.
    let mut best: Option<(usize, &(String, String))> = None;
    for task in &pending {
        let desc_lower = task.1.to_lowercase();
        let hits = words.iter().filter(|w| desc_lower.contains(w.as_str())).count();
        if hits == 0 {
            continue;
        }
        if best.map(|(s, _)| hits > s).unwrap_or(true) {
            best = Some((hits, task));
        }
    }

    let Some((_, (id, description))) = best else {
        // Return a useful payload so the model can re-call with the right keywords.
        let list = pending
            .iter()
            .take(10)
            .map(|(_, d)| format!("- {}", d))
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "No task matched '{}'. Pending tasks:\n{}\n\nCall complete_task again with keywords drawn from one of those exact descriptions.",
            search, list
        ));
    };

    {
        let conn = db.conn();
        conn.execute(
            "UPDATE action_items SET completed = 1, updated_at = datetime('now') WHERE id = ?1",
            [id],
        )
        .map_err(|e| format!("DB update failed: {}", e))?;
    }

    Ok(format!("Marked as complete: \"{}\"", description))
}

async fn update_task(args: &Value, db: &Arc<Database>) -> Result<String, String> {
    let search = args
        .get("search")
        .and_then(Value::as_str)
        .ok_or("Missing 'search'")?;
    let new_description = args.get("new_description").and_then(Value::as_str);
    let new_priority = args.get("new_priority").and_then(Value::as_str);
    let new_due_at = args.get("new_due_at").and_then(Value::as_str);

    if new_description.is_none() && new_priority.is_none() && new_due_at.is_none() {
        return Err("No fields to update".into());
    }

    let pattern = format!("%{}%", search.to_lowercase());
    let row: Option<(String, String)> = {
        let conn = db.conn();
        conn.query_row(
            "SELECT id, description FROM action_items
             WHERE LOWER(description) LIKE ?1
             ORDER BY completed ASC, created_at DESC LIMIT 1",
            [&pattern],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .ok()
    };

    let Some((id, original)) = row else {
        return Ok(format!("No task matching '{}' found.", search));
    };

    let conn = db.conn();
    if let Some(d) = new_description {
        conn.execute(
            "UPDATE action_items SET description = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![d, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(p) = new_priority {
        conn.execute(
            "UPDATE action_items SET priority = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![p, id],
        )
        .map_err(|e| e.to_string())?;
    }
    if let Some(d) = new_due_at {
        conn.execute(
            "UPDATE action_items SET due_at = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![d, id],
        )
        .map_err(|e| e.to_string())?;
    }

    Ok(format!("Updated task \"{}\"", original))
}

async fn list_tasks(args: &Value, db: &Arc<Database>) -> Result<String, String> {
    let filter = args
        .get("filter")
        .and_then(Value::as_str)
        .unwrap_or("pending");

    let where_clause = match filter {
        "all" => "",
        "completed" => "WHERE completed = 1",
        _ => "WHERE completed = 0",
    };

    let rows: Vec<(String, String, bool, Option<String>)> = {
        let conn = db.conn();
        let sql = format!(
            "SELECT id, description, completed, priority FROM action_items {}
             ORDER BY completed ASC, created_at DESC LIMIT 30",
            where_clause
        );
        let mut stmt = conn.prepare(&sql).map_err(|e| e.to_string())?;
        let rows: Vec<(String, String, bool, Option<String>)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    if rows.is_empty() {
        return Ok(format!("No {} tasks found.", filter));
    }

    let mut out = format!("{} {} task(s):\n", rows.len(), filter);
    for (_, desc, done, prio) in &rows {
        let mark = if *done { "[x]" } else { "[ ]" };
        let p = prio.as_deref().unwrap_or("medium");
        out.push_str(&format!("  {} ({}) {}\n", mark, p, desc));
    }
    Ok(out)
}

async fn update_memory_tool(
    args: &Value,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<String, String> {
    let search = args
        .get("search")
        .and_then(Value::as_str)
        .ok_or("Missing 'search'")?;
    let new_content = args
        .get("new_content")
        .and_then(Value::as_str)
        .ok_or("Missing 'new_content'")?;

    let row = find_memory(search, db, embedder).await?;
    let Some((id, original)) = row else {
        return Ok(format!("No memory matching '{}' found.", search));
    };

    {
        let conn = db.conn();
        conn.execute(
            "UPDATE memories SET content = ?1, updated_at = datetime('now') WHERE id = ?2",
            rusqlite::params![new_content, id],
        )
        .map_err(|e| format!("DB update failed: {}", e))?;
    }

    // Re-embed so chat retrieval finds the updated version
    let _ = rag::store_embedding(embedder, db, "memory", &id, new_content).await;

    Ok(format!(
        "Memory updated. Was: \"{}\" — now: \"{}\"",
        original, new_content
    ))
}

async fn delete_memory_tool(args: &Value, db: &Arc<Database>) -> Result<String, String> {
    let search = args
        .get("search")
        .and_then(Value::as_str)
        .ok_or("Missing 'search'")?;

    // Use word-AND search (no embedder needed for delete — keep it simple)
    let row: Option<(String, String)> = find_memory_by_words(search, db);

    let Some((id, content)) = row else {
        return Ok(format!("No memory matching '{}' found.", search));
    };

    {
        let conn = db.conn();
        conn.execute("DELETE FROM memories WHERE id = ?1", [&id])
            .map_err(|e| format!("DB delete failed: {}", e))?;
        let _ = conn.execute(
            "DELETE FROM embeddings WHERE entity_type = 'memory' AND entity_id = ?1",
            [&id],
        );
    }

    Ok(format!("Memory deleted: \"{}\"", content))
}

async fn list_memories_tool(args: &Value, db: &Arc<Database>) -> Result<String, String> {
    let search = args.get("search").and_then(Value::as_str);

    let rows: Vec<(String, String, String)> = {
        let conn = db.conn();
        let row_to_tuple = |row: &rusqlite::Row<'_>| -> rusqlite::Result<(String, String, String)> {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        };
        if let Some(s) = search {
            let pattern = format!("%{}%", s.to_lowercase());
            let mut stmt = conn
                .prepare(
                    "SELECT id, content, category FROM memories
                     WHERE is_dismissed = 0 AND LOWER(content) LIKE ?1
                     ORDER BY created_at DESC LIMIT 30",
                )
                .map_err(|e| e.to_string())?;
            let collected: Vec<(String, String, String)> = stmt
                .query_map([&pattern], row_to_tuple)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            collected
        } else {
            let mut stmt = conn
                .prepare(
                    "SELECT id, content, category FROM memories
                     WHERE is_dismissed = 0
                     ORDER BY created_at DESC LIMIT 20",
                )
                .map_err(|e| e.to_string())?;
            let collected: Vec<(String, String, String)> = stmt
                .query_map([], row_to_tuple)
                .map_err(|e| e.to_string())?
                .filter_map(|r| r.ok())
                .collect();
            collected
        }
    };

    if rows.is_empty() {
        let suffix = search
            .map(|s| format!(" matching '{}'", s))
            .unwrap_or_default();
        return Ok(format!("No memories{}.", suffix));
    }

    let mut out = format!("{} memory/memories:\n", rows.len());
    for (_, content, category) in &rows {
        out.push_str(&format!("  - [{}] {}\n", category, content));
    }
    Ok(out)
}

/// Serializes the create_memory critical section: search → LLM resolver →
/// DB write. Without this lock, two parallel `create_memory` calls (e.g.
/// from a tool-using chat turn and the post-conversation extractor running
/// concurrently) would both miss the duplicate (TOCTOU window during the
/// multi-second resolver call) and both insert.
static CREATE_MEMORY_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn create_memory(
    args: &Value,
    llm: &LlmClient,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<String, String> {
    let content = args
        .get("content")
        .and_then(Value::as_str)
        .ok_or("Missing 'content'")?;
    let category = args
        .get("category")
        .and_then(Value::as_str)
        .unwrap_or("system");

    dedup_or_create_memory(content, category, None, llm, db, embedder).await
}

/// Public entry point for "save a memory, with dedup". Used by both the
/// `create_memory` chat tool and the post-conversation processor — so
/// that both paths benefit from the LLM resolver and neither produces
/// duplicates of the other's output.
pub async fn dedup_or_create_memory(
    content: &str,
    category: &str,
    conversation_id: Option<&str>,
    llm: &LlmClient,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<String, String> {
    // Hold the dedup lock for the full search → resolver → write sequence.
    let _guard = CREATE_MEMORY_LOCK.lock().await;

    // Look for a near-duplicate. Threshold raised back to 0.70 — at 0.55
    // nomic-embed-text was firing on weakly-related facts and the resolver
    // (biased toward merge) produced Frankenstein memories. 0.70 is the
    // empirical "topically overlapping" floor for nomic-embed-text on
    // short factual sentences.
    let top_hit = rag::search(embedder, db, content, 1)
        .await
        .ok()
        .and_then(|hits| hits.into_iter().next())
        .filter(|h| h.entity_type == "memory" && h.score > 0.70);

    if let Some(hit) = top_hit {
        // Fetch the existing row's category for context.
        let existing_category: String = {
            let conn = db.conn();
            conn.query_row(
                "SELECT category FROM memories WHERE id = ?1",
                [&hit.entity_id],
                |row| row.get(0),
            )
            .unwrap_or_else(|_| "system".to_string())
        };

        match resolve_memory_dedup(
            llm,
            &existing_category,
            &hit.text,
            category,
            content,
        )
        .await
        {
            Ok(DedupAction::KeepExisting) => {
                return Ok(format!(
                    "Already remembered: \"{}\" (no duplicate created)",
                    hit.text.trim()
                ));
            }
            Ok(DedupAction::Merge(merged)) | Ok(DedupAction::Replace(merged)) => {
                {
                    let conn = db.conn();
                    conn.execute(
                        "UPDATE memories SET content = ?1, category = ?2,
                         updated_at = datetime('now') WHERE id = ?3",
                        rusqlite::params![merged, category, hit.entity_id],
                    )
                    .map_err(|e| format!("DB update failed: {}", e))?;
                }
                let _ = rag::store_embedding(
                    embedder,
                    db,
                    "memory",
                    &hit.entity_id,
                    &merged,
                )
                .await;
                return Ok(format!("Memory updated: \"{}\"", merged));
            }
            Ok(DedupAction::KeepBoth) => {
                // Fall through to insert.
            }
            Err(e) => {
                // Resolver failed — be conservative and just insert.
                log::warn!("Memory dedup resolver failed: {} — inserting new memory", e);
            }
        }
    }

    let id = uuid::Uuid::new_v4().to_string();
    {
        let conn = db.conn();
        // `manually_added=1` for chat-tool insertions (no conversation_id),
        // `manually_added=0` and conversation_id set for processor extractions.
        let manually_added: i64 = if conversation_id.is_some() { 0 } else { 1 };
        conn.execute(
            "INSERT INTO memories (id, content, category, conversation_id, manually_added)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![id, content, category, conversation_id, manually_added],
        )
        .map_err(|e| format!("DB insert failed: {}", e))?;
    }

    let _ = rag::store_embedding(embedder, db, "memory", &id, content).await;

    Ok(format!("Memory saved: \"{}\"", content))
}

enum DedupAction {
    KeepExisting,
    Merge(String),
    Replace(String),
    KeepBoth,
}

async fn resolve_memory_dedup(
    llm: &LlmClient,
    existing_category: &str,
    existing_text: &str,
    new_category: &str,
    new_text: &str,
) -> Result<DedupAction, String> {
    let prompt = prompts::MEMORY_DEDUP_PROMPT
        .replace("{existing_category}", existing_category)
        .replace("{existing_text}", existing_text)
        .replace("{new_category}", new_category)
        .replace("{new_text}", new_text);
    let raw = llm
        .chat(
            "You are a memory deduplication judge. Reply with ONLY a JSON object.",
            &prompt,
        )
        .await?;

    let cleaned = raw
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    let v: Value = serde_json::from_str(cleaned)
        .map_err(|e| format!("Resolver returned non-JSON: {} — raw: {}", e, raw))?;
    let action = v
        .get("action")
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Resolver missing action — raw: {}", raw))?;
    let merged = v
        .get("merged_content")
        .and_then(Value::as_str)
        .map(|s| s.trim().to_string());

    match action {
        "keep_existing" => Ok(DedupAction::KeepExisting),
        "merge" => merged
            .filter(|s| !s.is_empty())
            .map(DedupAction::Merge)
            .ok_or_else(|| "merge action requires merged_content".to_string()),
        "replace" => merged
            .filter(|s| !s.is_empty())
            .map(DedupAction::Replace)
            .ok_or_else(|| "replace action requires merged_content".to_string()),
        "keep_both" => Ok(DedupAction::KeepBoth),
        other => Err(format!("unknown action: {}", other)),
    }
}

// ============================================================
// Smart memory finder
//
// Strategy:
// 1. Try LIKE substring (whole search string)
// 2. If miss, split into words and require all words present in any order
// 3. If still miss, fall back to embedding similarity (top-1 above 0.5)
// ============================================================

fn split_keywords(s: &str) -> Vec<String> {
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2) // skip noise like "a", "of", "to"
        .map(String::from)
        .collect()
}

/// Try LIKE-substring then word-AND match. No embeddings, sync.
fn find_memory_by_words(search: &str, db: &Arc<Database>) -> Option<(String, String)> {
    let conn = db.conn();

    // Pass 1: substring match
    let pattern = format!("%{}%", search.to_lowercase());
    if let Ok(row) = conn.query_row(
        "SELECT id, content FROM memories
         WHERE is_dismissed = 0 AND LOWER(content) LIKE ?1
         ORDER BY created_at DESC LIMIT 1",
        [&pattern],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ) {
        return Some(row);
    }

    // Pass 2: all-words match
    let words = split_keywords(search);
    if words.is_empty() {
        return None;
    }

    let mut sql = String::from("SELECT id, content FROM memories WHERE is_dismissed = 0");
    let mut params: Vec<String> = Vec::new();
    for w in &words {
        sql.push_str(" AND LOWER(content) LIKE ?");
        params.push(format!("%{}%", w));
    }
    sql.push_str(" ORDER BY created_at DESC LIMIT 1");

    let mut stmt = conn.prepare(&sql).ok()?;
    let param_refs: Vec<&dyn rusqlite::ToSql> =
        params.iter().map(|s| s as &dyn rusqlite::ToSql).collect();
    stmt.query_row(&*param_refs, |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })
    .ok()
}

/// Try LIKE → word-AND → embedding-similarity. Async because of embeddings.
async fn find_memory(
    search: &str,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<Option<(String, String)>, String> {
    if let Some(hit) = find_memory_by_words(search, db) {
        return Ok(Some(hit));
    }

    // Pass 3: semantic search via embeddings
    let hits = rag::search(embedder, db, search, 3).await?;
    for hit in hits {
        if hit.entity_type == "memory" && hit.score > 0.5 {
            // Look up the memory by id to get the canonical content
            let conn = db.conn();
            let row = conn
                .query_row(
                    "SELECT id, content FROM memories WHERE id = ?1 AND is_dismissed = 0",
                    [&hit.entity_id],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
                )
                .ok();
            if row.is_some() {
                return Ok(row);
            }
        }
    }

    Ok(None)
}

// ============================================================
// Agentic search tools — let the LLM fetch context on demand
// ============================================================

async fn search_memories_tool(
    args: &Value,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or("Missing 'query'")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .map(|n| n.clamp(1, 20) as usize)
        .unwrap_or(5);

    let hits = rag::search(embedder, db, query, limit * 2)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;
    let memory_hits: Vec<_> = hits
        .into_iter()
        .filter(|h| h.entity_type == "memory")
        .take(limit)
        .collect();

    if memory_hits.is_empty() {
        return Ok(format!(
            "No memories found matching '{}'. The user hasn't recorded anything related yet.",
            query
        ));
    }

    let lines: Vec<String> = memory_hits
        .iter()
        .map(|h| format!("- ({:.0}%) {}", h.score * 100.0, h.text.trim()))
        .collect();
    Ok(format!(
        "Top {} memory match(es) for '{}':\n{}",
        memory_hits.len(),
        query,
        lines.join("\n")
    ))
}

async fn search_conversations_tool(
    args: &Value,
    db: &Arc<Database>,
    embedder: &Embedder,
) -> Result<String, String> {
    let query = args
        .get("query")
        .and_then(Value::as_str)
        .ok_or("Missing 'query'")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_i64)
        .map(|n| n.clamp(1, 10) as usize)
        .unwrap_or(5);

    let hits = rag::search(embedder, db, query, limit * 2)
        .await
        .map_err(|e| format!("Search failed: {}", e))?;
    let conv_hits: Vec<_> = hits
        .into_iter()
        .filter(|h| h.entity_type == "conversation")
        .take(limit)
        .collect();

    if conv_hits.is_empty() {
        return Ok(format!(
            "No past conversations found matching '{}'.",
            query
        ));
    }

    let lines: Vec<String> = conv_hits
        .iter()
        .map(|h| format!("- ({:.0}%) {}", h.score * 100.0, h.text.trim()))
        .collect();
    Ok(format!(
        "Top {} conversation match(es) for '{}':\n{}",
        conv_hits.len(),
        query,
        lines.join("\n")
    ))
}

async fn get_today_summary_tool(db: &Arc<Database>) -> Result<String, String> {
    let conn = db.conn();

    let conversations: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM conversations
             WHERE discarded = 0 AND date(started_at) = date('now', 'localtime')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let memories_today: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM memories
             WHERE is_dismissed = 0 AND date(created_at) = date('now', 'localtime')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let pending_tasks: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM action_items WHERE completed = 0",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    let completed_today: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM action_items
             WHERE completed = 1 AND date(updated_at) = date('now', 'localtime')",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);

    // Pull the actual descriptions of pending tasks (capped) so the model
    // can mention specifics if asked.
    let pending_list: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT description FROM action_items
                 WHERE completed = 0 ORDER BY created_at DESC LIMIT 5",
            )
            .map_err(|e| e.to_string())?;
        let rows: Vec<String> = stmt
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|e| e.to_string())?
            .filter_map(|r| r.ok())
            .collect();
        rows
    };

    let mut out = format!(
        "Today's snapshot:\n- {} new conversation(s) captured\n- {} new memory(ies) saved\n- {} task(s) completed today\n- {} pending task(s) total",
        conversations, memories_today, completed_today, pending_tasks
    );
    if !pending_list.is_empty() {
        out.push_str("\n\nPending tasks:");
        for d in &pending_list {
            out.push_str(&format!("\n- {}", d));
        }
    }
    Ok(out)
}
