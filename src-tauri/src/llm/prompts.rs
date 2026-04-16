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

/// Prompt to extract memories from a conversation transcript
pub const MEMORIES_PROMPT: &str = r#"You are analyzing a conversation transcript to extract memorable facts and learnings.

Extract memories as a JSON array. Each item:
{
  "content": "The fact or learning (max 15 words)",
  "category": "system | interesting"
}

Categories:
- "system": Facts about the user — preferences, habits, relationships, work details
  Example: "Prefers dark mode for all applications"
  Example: "Works on a project called Omniscient"
- "interesting": External knowledge, recommendations, insights with attribution
  Example: "Marcus recommended the book The Overstory"
  Example: "PipeWire replaced PulseAudio on modern Linux"

Rules:
- Maximum 2 system + 2 interesting memories per conversation
- Each memory must be a concrete, specific fact — not a vague summary
- Skip trivial things like greetings, filler words, or conversation mechanics
- Skip anything already obvious from context (like "user was talking")
- Return empty array [] if nothing worth remembering
- Return ONLY valid JSON array, no markdown"#;
