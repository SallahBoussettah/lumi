use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::RwLock;

/// OpenAI-compatible chat completion client (works with Ollama, OpenAI, etc.)
/// Model is hot-swappable via set_model().
pub struct LlmClient {
    base_url: String,
    model: RwLock<String>,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDef>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    /// Always serialized, even when empty. Some OpenAI-compatible servers
    /// (Ollama with qwen included) return 400 "invalid message content type:
    /// <nil>" if an assistant-with-tool_calls message omits the content field
    /// entirely. Sending an empty string is the safe default.
    #[serde(default)]
    pub content: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// For role="tool" — the tool_call_id this is responding to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// For role="tool" — the tool's name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }
    pub fn tool_result(
        tool_call_id: impl Into<String>,
        name: impl Into<String>,
        content: impl Into<String>,
    ) -> Self {
        Self {
            role: "tool".into(),
            content: content.into(),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: Some(name.into()),
        }
    }
}

/// A tool definition sent to the model.
#[derive(Serialize, Clone, Debug)]
pub struct ToolDef {
    #[serde(rename = "type")]
    pub kind: String, // always "function"
    pub function: ToolFunction,
}

#[derive(Serialize, Clone, Debug)]
pub struct ToolFunction {
    pub name: String,
    pub description: String,
    pub parameters: Value, // JSON schema
}

/// A tool call from the model in its response.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type", default = "default_tool_type")]
    pub kind: String,
    pub function: ToolCallFunction,
}

fn default_tool_type() -> String {
    "function".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToolCallFunction {
    pub name: String,
    /// JSON-encoded arguments string
    pub arguments: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

impl LlmClient {
    pub fn new(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            model: RwLock::new(model.to_string()),
            http: reqwest::Client::new(),
        }
    }

    pub fn ollama(model: &str) -> Self {
        Self::new("http://localhost:11434", model)
    }

    pub fn model(&self) -> String {
        self.model.read().unwrap().clone()
    }

    pub fn set_model(&self, model: &str) {
        *self.model.write().unwrap() = model.to_string();
        log::info!("LLM model switched to: {}", model);
    }

    /// Simple system+user one-shot chat (no tools), default temperature.
    pub async fn chat(&self, system_prompt: &str, user_prompt: &str) -> Result<String, String> {
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];
        self.chat_messages(&messages).await
    }

    /// Like `chat` but with an explicit temperature. The verification judge
    /// uses `temperature=0` for deterministic YES/NO answers.
    pub async fn chat_at_temp(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        temperature: f32,
    ) -> Result<String, String> {
        let messages = vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ];
        let msg = self
            .chat_messages_with_tools_at_temp(&messages, None, temperature)
            .await?;
        Ok(msg.content)
    }

    /// Send messages and return the response text (no tool calls).
    pub async fn chat_messages(&self, messages: &[ChatMessage]) -> Result<String, String> {
        let msg = self.chat_messages_with_tools(messages, None).await?;
        Ok(msg.content)
    }

    /// Send messages with optional tool defs. Returns the raw assistant message
    /// (which may contain tool_calls).
    pub async fn chat_messages_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
    ) -> Result<ChatMessage, String> {
        self.chat_messages_with_tools_at_temp(messages, tools, 0.3)
            .await
    }

    /// Underlying chat completion call with explicit temperature.
    pub async fn chat_messages_with_tools_at_temp(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
        temperature: f32,
    ) -> Result<ChatMessage, String> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatRequest {
            model: self.model(),
            messages: messages.to_vec(),
            temperature,
            stream: false,
            tools: tools.map(|t| t.to_vec()),
        };

        let resp = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("LLM error {}: {}", status, body));
        }

        let response: ChatResponse = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

        response
            .choices
            .into_iter()
            .next()
            .map(|c| c.message)
            .ok_or("No response from LLM".to_string())
    }

    /// Stream messages with optional tool defs. Calls `on_token` for every
    /// content delta from the model. Returns the final assembled message
    /// (including tool_calls if the model decided to call any).
    pub async fn chat_messages_stream<F>(
        &self,
        messages: &[ChatMessage],
        tools: Option<&[ToolDef]>,
        mut on_token: F,
    ) -> Result<ChatMessage, String>
    where
        F: FnMut(&str) + Send,
    {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatRequest {
            model: self.model(),
            messages: messages.to_vec(),
            temperature: 0.3,
            stream: true,
            tools: tools.map(|t| t.to_vec()),
        };

        let resp = self
            .http
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("LLM request failed: {}", e))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("LLM error {}: {}", status, body));
        }

        let mut content = String::new();
        let mut tool_calls: Vec<ToolCall> = Vec::new();
        let mut buffer = String::new();
        let mut stream = resp.bytes_stream();

        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| format!("Stream chunk error: {}", e))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            // SSE frames are separated by double newlines, each line begins
            // with `data: ` and contains JSON (or [DONE]).
            while let Some(idx) = buffer.find("\n\n") {
                let frame = buffer[..idx].to_string();
                buffer.drain(..idx + 2);

                for line in frame.lines() {
                    let line = line.trim_start();
                    if !line.starts_with("data: ") {
                        continue;
                    }
                    let payload = &line[6..];
                    if payload == "[DONE]" {
                        continue;
                    }
                    let parsed: Value = match serde_json::from_str(payload) {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    let Some(choice) = parsed
                        .get("choices")
                        .and_then(|c| c.as_array())
                        .and_then(|a| a.first())
                    else {
                        continue;
                    };
                    let Some(delta) = choice.get("delta") else {
                        continue;
                    };
                    if let Some(text) = delta.get("content").and_then(|c| c.as_str()) {
                        if !text.is_empty() {
                            content.push_str(text);
                            on_token(text);
                        }
                    }
                    // Accumulate tool calls — Ollama can send these in deltas
                    if let Some(calls) = delta.get("tool_calls").and_then(|c| c.as_array()) {
                        for call in calls {
                            // Each call has an index — append by index
                            let index = call.get("index").and_then(|v| v.as_u64()).unwrap_or(0)
                                as usize;
                            while tool_calls.len() <= index {
                                tool_calls.push(ToolCall {
                                    id: String::new(),
                                    kind: "function".to_string(),
                                    function: ToolCallFunction {
                                        name: String::new(),
                                        arguments: String::new(),
                                    },
                                });
                            }
                            let tc = &mut tool_calls[index];
                            if let Some(id) = call.get("id").and_then(|v| v.as_str()) {
                                tc.id = id.to_string();
                            }
                            if let Some(func) = call.get("function") {
                                if let Some(name) = func.get("name").and_then(|v| v.as_str()) {
                                    tc.function.name.push_str(name);
                                }
                                if let Some(args) = func.get("arguments").and_then(|v| v.as_str()) {
                                    tc.function.arguments.push_str(args);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(ChatMessage {
            role: "assistant".to_string(),
            content,
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
            name: None,
        })
    }

    pub async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/v1/models", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
