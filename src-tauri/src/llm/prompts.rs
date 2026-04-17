/// Prompt to extract structure from a conversation transcript
pub const STRUCTURE_PROMPT: &str = r#"You are analyzing a conversation transcript. Extract the following as JSON:

{
  "title": "Short title (max 10 words, Title Case)",
  "overview": "2-3 sentence summary of what was discussed",
  "emoji": "Single emoji that represents the topic",
  "category": "One of: work, personal, idea, meeting, learning, social, health, other"
}

Rules:
- Title should capture the core topic, not be generic like "Conversation" or "Discussion"
- Overview should mention specific topics, decisions, or outcomes
- Category should reflect the primary purpose of the conversation
- Return ONLY valid JSON, no markdown formatting"#;

/// Prompt to extract action items from a conversation transcript
pub const ACTION_ITEMS_PROMPT: &str = r#"You are analyzing a conversation transcript to extract action items and tasks.

Extract tasks as a JSON array. Each item:
{
  "description": "What needs to be done (6-15 words, starts with a verb)",
  "priority": "high | medium | low",
  "confidence": 0.0 to 1.0
}

Rules:
- Look for explicit requests: "remind me to", "I need to", "let's", "we should", "TODO"
- Look for implicit commitments: "I'll", "I'm going to", "next step is"
- Descriptions must start with an action verb (e.g., "Review", "Send", "Fix", "Schedule")
- High priority: due today or urgent language. Medium: this week. Low: no urgency
- Confidence: 0.9+ for explicit tasks, 0.7-0.9 for implicit, skip below 0.7
- Return empty array [] if no tasks found
- Return ONLY valid JSON array, no markdown"#;

/// Resolver prompt — used when create_memory finds a similar existing memory
/// (cosine > 0.75). The LLM picks one of four actions; the caller acts on the
/// returned JSON.
pub const MEMORY_DEDUP_PROMPT: &str = r#"You are deciding what to do with a NEW memory candidate that resembles an EXISTING memory.

EXISTING MEMORY (category: {existing_category}):
{existing_text}

NEW MEMORY CANDIDATE (category: {new_category}):
{new_text}

Pick exactly ONE action and reply with a single JSON object:
{
  "action": "keep_existing" | "merge" | "replace" | "keep_both",
  "merged_content": "(required for merge or replace; the final text to save, ≤30 words, present tense, full sentence)"
}

Decision guide (apply IN ORDER, stop at first match):

1. CORRECTION OR STRICT IMPROVEMENT → action=replace.
   New candidate fixes a typo, factual error, or supersedes the existing entirely (same subject, conflicting fact). Put the corrected sentence in `merged_content`.

2. EXACT DUPLICATE OR REPHRASING → action=keep_existing.
   New adds zero information. The existing memory already covers the same fact in different words. Omit `merged_content`.

3. SAME SUBJECT + COMPOSABLE FACTS → action=merge.
   Both facts are about the SAME subject AND they fit naturally into ONE sentence without listing or "and also" gymnastics (e.g. job + tool used at that job). If the result reads like a stuffed list rather than one fact, prefer keep_both. Output the merged sentence in `merged_content`.

4. EVERYTHING ELSE → action=keep_both.
   Different subjects, OR same subject with independent facts that would feel forced together. WHEN IN DOUBT, choose keep_both — two clean memories beat one Frankenstein memory.

Examples (one per action):
- merge:        existing="X works at company Y", new="X uses Python at work" → merged_content="X works at company Y using Python"
- replace:      existing="X's nickname is wrong-spelling", new="X's nickname is correct-spelling" → merged_content="X's nickname is correct-spelling"
- keep_both:    existing="A recommended the book Foo", new="X is reading the book Foo" → (different subjects performing different actions)
- keep_existing: existing="Prefers dark mode", new="Likes dark themes" → (same fact, different words)

Return ONLY the JSON object, no markdown.
"#;

/// Prompt to extract memories from a conversation transcript.
///
/// `{existing_memories}` is replaced at runtime with the user's recent
/// memories (or "(none yet)") so the model can avoid recreating duplicates.
pub const MEMORIES_PROMPT: &str = r#"You read a conversation transcript and extract at most 2 NEW memories worth keeping. Most conversations should yield 0 or 1.

Output: a JSON array. Each item:
{
  "content": "Direct factual statement, present tense, full sentence, ≤25 words",
  "category": "system" or "interesting"
}

# CATEGORIZATION DECISION TREE

For each candidate fact, ask in order:

Q1. Is this a fact ABOUT the user — their identity, preferences, habits, relationships, possessions, plans, work, location, body, beliefs?
    YES → category = "system". Write in third person ("Works as…", "Prefers…", "Lives in…").
    NO  → continue to Q2.

Q2. Is this an external claim or recommendation the user heard from someone, OR a piece of world knowledge worth remembering, AND it includes who said it / where it's from?
    YES → category = "interesting". Lead with the source ("Marcus recommended…", "Per the docs…").
    NO  → SKIP. Do not include it.

# WHAT TO ALWAYS SKIP

- Greetings, filler, conversation mechanics ("Hi there", "as I was saying")
- Obvious meta-facts ("user is talking", "user asked a question")
- Anything the user said only to test the assistant or in passing
- Anything already covered by an existing memory below
- Vague summaries ("had a productive conversation")
- Hypotheticals or guesses ("might want to", "could be interested in")

# BANNED LANGUAGE

Do not use these hedging words in `content`: potentially, might, could be, appears to, seems to, presumably, likely, perhaps, maybe, possibly. Memories are facts, not guesses. If you'd need a hedge, SKIP the memory.

# EXAMPLES

Conversation: "I'm Salah, I work as a full stack developer using Tauri. My friend Marcus said I should read The Overstory."
Output:
[
  { "content": "Salah works as a full stack developer using Tauri", "category": "system" },
  { "content": "Marcus recommended the book The Overstory", "category": "interesting" }
]

Conversation: "Hey Lumi, what time is it? Oh and remind me to email John tomorrow."
Output: []
(time question is mechanics; the email is a TASK, not a memory.)

Conversation: "I think I might want to learn Rust eventually."
Output: []
("I think I might" = hedge, not a fact.)

Conversation: "I'm pescatarian, not vegetarian — I eat fish."
Output:
[
  { "content": "Is pescatarian — eats fish but no other meat", "category": "system" }
]

# EXISTING MEMORIES (do not duplicate these)

{existing_memories}

# OUTPUT

Return ONLY a valid JSON array. No markdown, no commentary, no leading/trailing text. Empty array `[]` if nothing qualifies.
"#;
