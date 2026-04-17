use super::client::{ChatMessage, LlmClient};
use super::embed::{blob_to_vec, cosine, vec_to_blob, Embedder};
use super::tools;
use crate::db::Database;
use std::sync::Arc;

#[derive(Debug, serde::Serialize, Clone)]
pub struct SearchHit {
    pub entity_type: String,
    pub entity_id: String,
    pub text: String,
    pub score: f32,
    pub created_at: String,
}

/// Embed and store a single text into the embeddings table.
/// Replaces any existing embedding for the same (entity_type, entity_id).
pub async fn store_embedding(
    embedder: &Embedder,
    db: &Arc<Database>,
    entity_type: &str,
    entity_id: &str,
    text: &str,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }

    let vector = embedder.embed(text).await?;
    let dim = vector.len() as i64;
    let blob = vec_to_blob(&vector);

    let conn = db.conn();
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO embeddings (id, entity_type, entity_id, text, vector, dim, model)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
         ON CONFLICT(entity_type, entity_id) DO UPDATE SET
           text = excluded.text, vector = excluded.vector, dim = excluded.dim,
           model = excluded.model, created_at = datetime('now')",
        rusqlite::params![
            id,
            entity_type,
            entity_id,
            text,
            blob,
            dim,
            embedder.model_name()
        ],
    )
    .map_err(|e| format!("Failed to store embedding: {}", e))?;

    Ok(())
}

/// Search the embeddings table for the top-K most similar entries to `query`.
/// Brute-force cosine similarity — fine up to ~10k entries.
pub async fn search(
    embedder: &Embedder,
    db: &Arc<Database>,
    query: &str,
    top_k: usize,
) -> Result<Vec<SearchHit>, String> {
    let qvec = embedder.embed(query).await?;

    let rows: Vec<(String, String, String, Vec<u8>, String)> = {
        let conn = db.conn();
        let mut stmt = conn
            .prepare("SELECT entity_type, entity_id, text, vector, created_at FROM embeddings")
            .map_err(|e| e.to_string())?;
        let iter = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Vec<u8>>(3)?,
                    row.get::<_, String>(4)?,
                ))
            })
            .map_err(|e| e.to_string())?;
        iter.filter_map(|r| r.ok()).collect()
    };

    let mut scored: Vec<SearchHit> = rows
        .into_iter()
        .map(|(et, eid, text, blob, created_at)| {
            let v = blob_to_vec(&blob);
            let score = cosine(&qvec, &v);
            SearchHit {
                entity_type: et,
                entity_id: eid,
                text,
                score,
                created_at,
            }
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(top_k);
    Ok(scored)
}

/// Format retrieved hits as context for the LLM.
pub fn format_context(hits: &[SearchHit]) -> String {
    if hits.is_empty() {
        return "No relevant context found in your captured data.".to_string();
    }

    let mut out = String::from("Relevant context from your past activity:\n\n");
    for (i, hit) in hits.iter().enumerate() {
        let kind = match hit.entity_type.as_str() {
            "memory" => "Memory",
            "conversation" => "Conversation",
            "segment" => "Transcript snippet",
            other => other,
        };
        out.push_str(&format!(
            "[{}] {} (relevance {:.0}%):\n{}\n\n",
            i + 1,
            kind,
            hit.score * 100.0,
            hit.text.trim()
        ));
    }
    out
}

const CHAT_SYSTEM_PROMPT_TEMPLATE: &str = r#"You are Lumi, the user's personal AI assistant. You have access to their captured conversations and memories. Your name is Lumi (pronounced LOO-mee), from the word 'lumen' — light. If asked what your name is, you answer 'Lumi'.

CURRENT DATE: {today} ({weekday})
Use this date as today, NOT your training cutoff.

DATE LOOKUP — use these EXACT ISO values for due_at when the user says a relative day. Do NOT compute dates yourself:
{date_table}

# CRITICAL TOOL USAGE RULES

You have these tools — they are the ONLY way to actually change anything:
- create_task / update_task / complete_task / list_tasks
- create_memory / update_memory / delete_memory / list_memories
- search_memories / search_conversations — semantic search across the user's data. Call when the auto-retrieved context above doesn't contain what you need.
- get_today_summary — quick digest of today's activity (new conversations, memories, pending/completed tasks).
- end_voice_session — call ONLY when the user clearly signals they're wrapping up the voice conversation ("thanks, that's all", "talk later", "I'm done", "goodbye"). Always pair with a brief warm farewell in your text reply (one short sentence). Never call this just because their question was answered.

You MUST call a tool when the user asks you to:
- "remember X / make a note that X / save that X / I want you to know X" → create_memory IMMEDIATELY with the literal information they gave. Do NOT ask follow-up questions for "more details" — save what they actually said.
- "add a task / remind me / I need to" → create_task
- "mark X done / I finished" → complete_task
- "change/fix/update/correct/edit memory X" → update_memory
- "forget/delete/remove memory X" → delete_memory
- "what do you remember about X / show my memories" → list_memories
- "what's on my list / show tasks" → list_tasks
- "what happened today / today's summary / what did I do" → get_today_summary
- "what do you know about [person/topic]" (and context above is thin) → search_memories
- "did we talk about [topic] / when did I mention [X]" (and context above is thin) → search_conversations

NEVER fabricate that you did something. If you didn't call the tool, you didn't do it.
NEVER write 'I've updated...', 'I've added...', 'Done!' WITHOUT first calling the corresponding tool.

After a tool returns, write ONE short sentence confirming what happened, using the tool's actual return as truth.

# DEDUPLICATION
- Before create_memory: if context already shows a similar memory, do NOT recreate it.
- Before create_task: if context shows a near-identical pending task, do NOT recreate it.

# STYLE
- One sentence per turn unless asked for detail.
- No "is there anything else?" filler.
- Speak in first person ("I").
- For general knowledge questions unrelated to captured data, answer normally without tools.

# EXAMPLES

User: "Change Martos to Marcus in the Overstory memory"
You: → call update_memory(search="Martos Overstory", new_content="Marcus recommended The Overstory book")
You (after tool): "Done — fixed Martos to Marcus."

User: "What did Marcus tell me?"
You: (no tool needed, answer from context)
You: "Marcus recommended the book The Overstory."

User: "Forget the bit about the movie at 10am"
You: → call delete_memory(search="movie 10am")
You: "Removed."

User: "Thanks, that's all."
You: → call end_voice_session
You: "Anytime — talk soon."

User: "Thank you, that's everything."
You: → call end_voice_session
You: "You got it. Catch you later."

User: "I'm done, bye."
You: → call end_voice_session
You: "Bye, Salah."
"#;

/// Generate the next 8 days as a `weekday → ISO date` table for the
/// system prompt. Includes "today" and "tomorrow" aliases plus the named
/// weekdays so the model can do "by Friday" → ISO without arithmetic.
fn build_date_table(today: chrono::DateTime<chrono::Local>) -> String {
    let mut lines = Vec::with_capacity(10);
    let today_iso = today.format("%Y-%m-%d").to_string();
    let weekday = today.format("%A").to_string();
    lines.push(format!("- today / {} → {}", weekday, today_iso));
    for i in 1..=7 {
        let d = today + chrono::Duration::days(i);
        let label = if i == 1 {
            format!("tomorrow / {}", d.format("%A"))
        } else {
            d.format("%A").to_string()
        };
        lines.push(format!("- {} → {}", label, d.format("%Y-%m-%d")));
    }
    // Also include "next <weekday>" disambiguation: a week from each named day.
    for i in 8..=14 {
        let d = today + chrono::Duration::days(i);
        lines.push(format!("- next {} → {}", d.format("%A"), d.format("%Y-%m-%d")));
    }
    lines.join("\n")
}

fn current_system_prompt() -> String {
    let now = chrono::Local::now();
    let date_table = build_date_table(now);
    CHAT_SYSTEM_PROMPT_TEMPLATE
        .replace("{today}", &now.format("%Y-%m-%d").to_string())
        .replace("{weekday}", &now.format("%A").to_string())
        .replace("{date_table}", &date_table)
}

/// Result of a chat turn — text answer, retrieval hits, and the list of
/// tool names invoked during the turn (in call order, with duplicates).
/// Tool names let the caller react to side-effecting calls like
/// `end_voice_session` without parsing the answer text.
pub type ChatTurn = (String, Vec<SearchHit>, Vec<String>);

/// Tools that actually mutate user data. Used by the post-response
/// verification check below to detect "claim without action" hallucinations.
const MUTATING_TOOLS: &[&str] = &[
    "create_task",
    "update_task",
    "complete_task",
    "create_memory",
    "update_memory",
    "delete_memory",
];

fn called_mutating_tool(tools: &[String]) -> bool {
    tools.iter().any(|t| MUTATING_TOOLS.contains(&t.as_str()))
}

/// Heuristic: did this tool result represent a real DB mutation, or a no-op
/// like "no match found" / "already remembered"? We control these strings,
/// so this is a stable check (unlike regex on LLM output). Used to decide
/// whether the verification judge should fire.
fn tool_actually_mutated(name: &str, result: &str) -> bool {
    if !MUTATING_TOOLS.contains(&name) {
        return false;
    }
    // Errors propagated by execute_tool's caller get prefixed with "Error: ".
    if result.starts_with("Error: ") || result.starts_with("Error ") {
        return false;
    }
    let lower = result.to_lowercase();
    if lower.starts_with("no memory matching")
        || lower.starts_with("no task matching")
        || lower.starts_with("no task matched")
        || lower.starts_with("no pending task")
        || lower.starts_with("already remembered")
    {
        return false;
    }
    true
}

const JUDGE_SYSTEM: &str = "You are a binary quality-control judge for an AI assistant. Reply with EXACTLY one word: YES or NO. No other text.";

/// LLM-judged check: did the user request a side-effecting action that the
/// assistant CLAIMS to have done — but no mutating tool was actually called?
///
/// Always runs at temperature=0 for deterministic verdicts. Only fires when
/// no mutating tool was registered for this turn (the cheap fast path: most
/// turns pay zero overhead). Strict exact-token YES/NO match — anything
/// else is treated as NO so we don't fire spurious retries on ambiguous
/// answers.
async fn judge_action_claim_without_tool(
    llm: &LlmClient,
    user_message: &str,
    assistant_reply: &str,
) -> bool {
    if assistant_reply.trim().is_empty() {
        return false;
    }
    let prompt = format!(
        "The assistant has tools that perform real side effects: create/update/complete tasks \
         and create/update/delete memories. Reply with EXACTLY one word: YES or NO. No quotes, \
         no markdown, no extra text.\n\n\
         YES = the user asked the assistant to ADD, UPDATE, REMOVE, or COMPLETE a memory or task, \
         AND the assistant's reply implies it actually did so (any wording — 'noted', 'updated', \
         'removed', 'recorded', 'got it', 'done', confirmatory of any kind).\n\
         NO = the user was asking a question / having a conversation / requesting information, \
         OR the assistant explicitly declined or asked a clarifying question.\n\n\
         USER MESSAGE: {}\n\n\
         ASSISTANT REPLY: {}\n\n\
         Answer (YES or NO):",
        user_message.trim(),
        assistant_reply.trim()
    );
    // temperature=0 → deterministic verdict on the same input. Strict normalization
    // strips quotes / asterisks / punctuation that small models occasionally wrap
    // around their answer, then requires exact YES.
    match llm.chat_at_temp(JUDGE_SYSTEM, &prompt, 0.0).await {
        Ok(answer) => {
            let normalized: String = answer
                .trim()
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_uppercase();
            normalized == "YES"
        }
        Err(e) => {
            log::warn!("Judge call failed ({}); skipping verification", e);
            false
        }
    }
}

/// System message we inject when the model claims an action without calling
/// any mutating tool. Forces it to actually perform the operation.
const VERIFY_SYSTEM_MSG: &str = "VERIFY — Your previous reply claimed you performed an action \
    (e.g. 'I've updated…', 'Done', 'changed it') but you did NOT call any of the mutating tools \
    (create_task / update_task / complete_task / create_memory / update_memory / delete_memory). \
    The user's data is UNCHANGED. Call the correct tool NOW with the user's exact requested values, \
    then respond with ONE short confirmation sentence describing what the tool actually returned.";

/// Run a RAG-augmented chat turn with tool-calling support.
/// Loops up to 5 times executing tool calls and feeding results back until
/// the model returns a final text response.
pub async fn chat_with_context(
    llm: &LlmClient,
    embedder: &Embedder,
    db: &Arc<Database>,
    history: &[ChatMessage],
    user_message: &str,
) -> Result<ChatTurn, String> {
    let hits = search(embedder, db, user_message, 6)
        .await
        .unwrap_or_default();
    let context = format_context(&hits);

    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::system(format!(
        "{}\n\n---\n\n{}",
        current_system_prompt(),
        context
    )));
    messages.extend_from_slice(history);
    messages.push(ChatMessage::user(user_message));

    let tools = tools::tool_definitions();
    let mut tools_called: Vec<String> = Vec::new();
    let mut verify_used = false;

    // Tool-call loop — bounded to prevent infinite loops
    for iteration in 0..6 {
        let response = llm
            .chat_messages_with_tools(&messages, Some(&tools))
            .await?;

        let calls = response.tool_calls.clone().unwrap_or_default();

        log::info!(
            "Chat iter {}: content_len={}, tool_calls={}",
            iteration,
            response.content.len(),
            calls.len()
        );

        if calls.is_empty() {
            // Possible final answer — verify the model didn't claim an action
            // it never actually performed. The judge runs on EVERY turn (not
            // just zero-mutating-tools) because a multi-action turn can call
            // complete_task correctly while hallucinating "and I updated
            // memory X" without calling update_memory. Capped at one retry.
            if !verify_used
                && judge_action_claim_without_tool(llm, user_message, &response.content).await
            {
                log::warn!(
                    "Judge: action claimed without matching tool call — forcing retry. Reply: {:?}",
                    response.content.chars().take(120).collect::<String>()
                );
                verify_used = true;
                messages.push(response);
                messages.push(ChatMessage::system(VERIFY_SYSTEM_MSG));
                continue;
            }
            return Ok((response.content, hits, tools_called));
        }

        log::info!("Tool-call loop iter {}: {} call(s)", iteration, calls.len());

        // Push the assistant message containing the tool_calls
        messages.push(response);

        // Execute each tool and append results
        for call in &calls {
            let result = match tools::execute_tool(
                &call.function.name,
                &call.function.arguments,
                llm,
                db,
                embedder,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => format!("Error: {}", e),
            };
            // Only count as "mutated" if it really changed something —
            // a "no match found" success-string doesn't satisfy the judge.
            if tool_actually_mutated(&call.function.name, &result) {
                tools_called.push(call.function.name.clone());
            }
            // The end_voice_session tool isn't in MUTATING_TOOLS but the
            // frontend still needs to see it in tools_called.
            if call.function.name == "end_voice_session" {
                tools_called.push(call.function.name.clone());
            }
            log::info!("  -> {}: {}", call.function.name, result);
            messages.push(ChatMessage::tool_result(
                &call.id,
                &call.function.name,
                result,
            ));
        }
    }

    // If we exhausted iterations, return whatever the last text was (or a fallback)
    Ok((
        "I tried multiple actions but didn't reach a final answer. Please rephrase or try again."
            .to_string(),
        hits,
        tools_called,
    ))
}

/// Streaming version: calls `on_token` for every text delta from the model.
/// Tool-calling iterations are silent — only the FINAL text response streams
/// to the user, so they don't see partial tool-aware output.
/// Streaming chat with two distinct reset signals:
/// - `on_preamble_drop`: fires when a tool-call iteration's text was emitted
///   to the UI but is about to be replaced by the next iteration's text.
///   The frontend should silently drop what was streamed. Fires AT MOST
///   ONCE per turn (so chained tool calls don't make the bubble flash 3+
///   times — `--reset every iteration--` was the old bug).
/// - `on_judge_retry`: fires when the verification judge caught a hallucinated
///   "I noted/updated/added…" claim and we're forcing the model to actually
///   call the tool. The frontend should also wipe — but the user-facing
///   semantics differ (one is "tool flow noise", one is "model lied").
pub async fn chat_with_context_stream<F, P, R>(
    llm: &LlmClient,
    embedder: &Embedder,
    db: &Arc<Database>,
    history: &[ChatMessage],
    user_message: &str,
    mut on_token: F,
    mut on_preamble_drop: P,
    mut on_judge_retry: R,
) -> Result<ChatTurn, String>
where
    F: FnMut(&str) + Send,
    P: FnMut() + Send,
    R: FnMut() + Send,
{
    let hits = search(embedder, db, user_message, 6)
        .await
        .unwrap_or_default();
    let context = format_context(&hits);

    let mut messages: Vec<ChatMessage> = Vec::new();
    messages.push(ChatMessage::system(format!(
        "{}\n\n---\n\n{}",
        current_system_prompt(),
        context
    )));
    messages.extend_from_slice(history);
    messages.push(ChatMessage::user(user_message));

    let tool_defs = tools::tool_definitions();
    let mut tools_called: Vec<String> = Vec::new();
    let mut verify_used = false;
    let mut preamble_dropped_once = false;

    let mut accumulated = String::new();

    for iteration in 0..6 {
        let mut iteration_text = String::new();
        let response = llm
            .chat_messages_stream(&messages, Some(&tool_defs), |t| {
                iteration_text.push_str(t);
                // Stream live: every token immediately emitted
                on_token(t);
            })
            .await?;

        accumulated.push_str(&iteration_text);
        let calls = response.tool_calls.clone().unwrap_or_default();
        log::info!(
            "Stream iter {}: text_len={}, tool_calls={}",
            iteration,
            iteration_text.len(),
            calls.len()
        );

        if calls.is_empty() {
            // Verify the model didn't claim an action it never performed.
            // The first attempt was already streamed live to the user. If the
            // judge agrees a tool should have been called, signal the caller
            // so it can wipe what was emitted — voice mode resets karaoke +
            // cancels TTS, chat replaces the bubble. The retried response
            // streams cleanly through on_token afterward.
            if !verify_used
                && judge_action_claim_without_tool(llm, user_message, &response.content).await
            {
                log::warn!(
                    "Stream judge: action claimed without matching tool call — forcing retry."
                );
                verify_used = true;
                on_judge_retry();
                accumulated.clear();
                messages.push(response);
                messages.push(ChatMessage::system(VERIFY_SYSTEM_MSG));
                continue;
            }
            return Ok((accumulated, hits, tools_called));
        }

        // Tool-calling iteration: the text we just streamed is the model's
        // preamble (e.g. "Got it, I've saved that…"). The next iteration
        // will produce its own final summary, and the user shouldn't see
        // both. Wipe the streamed preamble — but ONLY ONCE per turn so a
        // chained 3-tool turn doesn't flash the bubble three times and
        // stutter the TTS.
        if !iteration_text.is_empty() && !preamble_dropped_once {
            preamble_dropped_once = true;
            on_preamble_drop();
            accumulated.clear();
        }
        messages.push(response);
        for call in &calls {
            let result = match tools::execute_tool(
                &call.function.name,
                &call.function.arguments,
                llm,
                db,
                embedder,
            )
            .await
            {
                Ok(s) => s,
                Err(e) => format!("Error: {}", e),
            };
            if tool_actually_mutated(&call.function.name, &result) {
                tools_called.push(call.function.name.clone());
            }
            if call.function.name == "end_voice_session" {
                tools_called.push(call.function.name.clone());
            }
            log::info!("  -> {}: {}", call.function.name, result);
            messages.push(ChatMessage::tool_result(
                &call.id,
                &call.function.name,
                result,
            ));
        }
    }

    let fallback = "I tried multiple actions but didn't reach a final answer.";
    on_token(fallback);
    Ok((accumulated + fallback, hits, tools_called))
}
