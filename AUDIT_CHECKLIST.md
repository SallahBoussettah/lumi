# Lumi Codebase Audit

Findings from a 4-specialist team review (April 2026):
- **rust-systems** — Rust backend, async, Tauri integration
- **frontend-react** — React/TS, hooks, UX
- **ai-prompts** — LLM prompts, tool design, agentic loops
- **reliability** — cross-cutting, races, edge cases

Issues are ranked by **severity × number of independent reviewers who flagged it**.

## Status — 2026-04-17

- ✅ All 5 critical issues addressed
- ✅ All 8 high-severity issues addressed (one resolved as by-design)
- ⏳ Medium-severity items deferred to a follow-up polish round

---

## 🔴 Critical (must fix)

### C1 — Conversations stuck in `'processing'` forever
**Flagged by:** rust-systems, reliability, frontend-react
- `stop_recording` sets `status='processing'`, then expects the frontend to call `process_conversation_cmd`.
- **`FloatingBar.tsx:262-281` never calls `processConversation`** — recordings from the floating bar are transcribed but never extracted.
- Startup cleanup at `lib.rs:1592` only handles `'in_progress'`, not `'processing'`.
- No UI to retry. Real silent data loss.
- **Fix:** Either auto-process on Rust side after `stop_recording`, or have FloatingBar use `cancelRecording` (it's chat input, not a capture). Plus: extend startup cleanup to handle `'processing'`.
- [x] Fixed

### C2 — `chat-token` event race — early tokens dropped
**Flagged by:** rust-systems, frontend-react, reliability, ai-prompts
- Pattern `await listen("chat-token") → await chatSendStream()`. Tauri events are fire-and-forget with no replay.
- **Investigation:** Both ChatPage and FloatingBar already do `await listen()` before `chatSendStream`. Tauri's `listen()` Promise resolves only after the listener is fully registered with the backend, so the race is theoretical given the current pattern. FloatingBar's "stuck on thinking" is already mitigated by `setMode("answer")` on invoke return (`FloatingBar.tsx:229`).
- **Action:** Documented; no code change needed. If a real race surfaces under load, upgrade to Tauri `Channel<T>`.
- [x] Resolved (pattern already correct)

### C3 — Single SQLite connection mutex serializes the whole app
**Flagged by:** rust-systems, frontend-react, reliability
- `Database { conn: Mutex<Connection> }` (`db/mod.rs:7-32`) — every DB-touching command queues behind every other.
- `transcribe_pending` holds the lock for multi-second whisper passes; meanwhile every UI poll stalls.
- WAL mode gains nothing with one connection.
- **Fix:** `r2d2_sqlite` or `deadpool-sqlite` pool. Pair with frontend `inFlight` guards.
- [x] Fixed

### C4 — Streaming preamble wipe (`on_retry`) over-fires per tool iteration
**Flagged by:** rust-systems, frontend-react, ai-prompts, reliability
- `rag.rs:526-529` calls `on_retry()` after EVERY tool-calling iteration's preamble.
- Chained tool calls (3 in a turn) → bubble flickers 3 times, `tts.stop()` cuts mid-sentence, voice karaoke desyncs (`nextIndex` resets to 0 while `voiceClipIndex` is bound to stale subscriber).
- **Fix:** Split into two callbacks (`on_preamble_drop` vs `on_judge_retry`). Only fire `on_preamble_drop` once per turn — or eliminate the per-iteration wipe by deferring TTS until the final iteration.
- [x] Fixed

### C5 — Memory dedup misfires (over-merge + TOCTOU race)
**Flagged by:** ai-prompts, rust-systems, reliability
- Cosine threshold `0.55` (`tools.rs:681`) is too low for nomic-embed-text → resolver fires on weakly-related facts.
- Prompt's "Default to merge when in doubt" → Frankenstein memories like "Salah works at Acme using Tauri and likes coffee".
- TOCTOU: `rag::search` → multi-second resolver LLM call → DB write. Concurrent `create_memory` calls see "no duplicate" and both insert.
- `processor.rs:107-112` `extract_memories` doesn't dedup AT ALL.
- **Fix:** Threshold 0.55 → 0.70, soften prompt away from merge-bias, wrap search+resolve+write in a serialized critical section, run dedup in `extract_memories` too.
- [x] Fixed

---

## 🟠 High

### H1 — `end_voice_session` is dead code
**Flagged by:** frontend-react, ai-prompts
- LLM is taught to call it (3 examples in system prompt). Rust pushes it into `tools_called`. **Frontend never reads it.**
- Voice mode only ends on manual mic toggle.
- **Fix:** Five-line frontend handler — when `result.tools_called.includes("end_voice_session")` after a voice turn, set `voiceMode=false`.
- [x] Fixed

### H2 — `cancelRecording` on ChatPage unmount destroys recordings started elsewhere
**Flagged by:** reliability
- `ChatPage.tsx:411-419` unconditionally calls `cancelRecording()` on unmount.
- Start a recording from FloatingBar → navigate to Chat → navigate to Settings → recording deleted. Real data loss.
- **Fix:** Track recording ownership; only cancel what this component started.
- [x] Fixed

### H3 — Voice mode never persists conversations
**Flagged by:** reliability
- Voice mode's silence-stop calls `cancelRecording`. Voice turns are ephemeral by design.
- **Decision:** This is intentional — voice mode IS chat, and the chat-message history IS persisted in `chat_sessions`/`messages` tables. The `conversations` table is for explicit captures (mic recordings via the Conversations page). Voice input ≠ recording.
- **Action:** Document the design distinction; no code change needed.
- [x] Resolved (by-design)

### H4 — Sync `Mutex` held across `.await` in async commands
**Flagged by:** rust-systems
- `tauri::State<'_, Arc<std::sync::Mutex<T>>>` locked across awaits. `MutexGuard` is `!Send` → multi-thread Tokio runtime stalls.
- **Fix:** Swap to `tokio::sync::Mutex` for state held across awaits, or scope every lock to a `{ }` that drops before the await.
- [x] Fixed

### H5 — FK errors silently swallowed
**Flagged by:** rust-systems
- `let _ = conn.execute(...)` everywhere in `lib.rs`. If a chat session is deleted concurrently, assistant messages vanish silently.
- **Fix:** Propagate as `Result<_, String>`; log at `warn` minimum.
- [x] Fixed

### H6 — Verification judge skips when ANY mutating tool fired
**Flagged by:** ai-prompts
- `rag.rs:379, 503` — judge only fires if `tools_called` is empty.
- Model can call `complete_task` correctly, then claim "and updated memory X" without calling `update_memory` — judge skipped.
- Judge runs at default `temp=0.3`, gets non-deterministic verdicts.
- **Fix:** Run judge on every non-trivial turn. Set `temp=0`. Require exact YES/NO match.
- [x] Fixed

### H7 — AudioContext leak on Settings voice preview
**Flagged by:** frontend-react
- `new TtsPlayer()` per click in `SettingsPage.tsx:92-112`; never closed.
- Chromium caps ~6 concurrent contexts → silent failure after a few previews.
- **Fix:** Module-level shared AudioContext (`suspend()`/`resume()` only, never close).
- [x] Fixed

### H8 — Whisper transcribe contention drops audio
**Flagged by:** reliability, rust-systems
- Two pollers (`transcribe_pending` 400ms + `transcribe_partial` 1.2s) plus the cpal real-time callback contend for `speech_buffer`.
- `transcribe_partial` clones a multi-MB Vec under lock.
- cpal callbacks have ms-scale deadlines → missed = dropped audio.
- **Fix:** Move clone outside lock, gate partial poll while pending transcribe is in flight.
- [x] Fixed

---

## 🟡 Medium (worth doing later)

- **No single-instance lock** — two Lumi instances each wipe each other's `'in_progress'` rows on startup. `tauri-plugin-single-instance` fixes it.
- **Resampler `remainder` never consumed** (`capture.rs:166`) — partial audio chunks dropped silently.
- **TTS sidecar orphans on hard kill** — port stays bound, next launch can't see logs (`Stdio::null()`).
- **`unsafe impl Send for cpal::Stream`** — works on Linux ALSA, undefined on macOS/Windows.
- **Whisper `.bin.part` orphans** — flaky network = 1.5GB×N disk waste; no resume.
- **`tool_actually_mutated` string-prefix matching** — fragile, breaks if any tool changes its success message. Should be `enum ToolOutcome { Mutated, NoOp, Error }`.
- **`complete_task` returns Err on no-match while `update_task` returns Ok** — inconsistent, model retry-spams.
- **`get_today_summary` doesn't include due dates / priorities** — model can't answer "what's due today" after calling it.
- **`MEMORIES_PROMPT` SKIP-on-hedge rule** is too aggressive — discards legit observations like "often works late on Fridays".
- **`format_context` no per-type quota** — memories dominate, conversations starved.
- **Two `isRecording` pollers disagree by 1.5s** — Sidebar vs App.
- **`cleanForTts` regex chain on every render** — useMemo per message.
- **`Embedder` constructed per-call in processor** — spurious HTTP client churn.
- **Per-step status columns on conversation processing** — so reprocess can resume rather than wipe.
