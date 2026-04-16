use serde::{Deserialize, Serialize};

/// OpenAI-compatible chat completion client (works with Ollama, OpenAI, etc.)
pub struct LlmClient {
    base_url: String,
    model: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
    temperature: f32,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
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
            model: model.to_string(),
            http: reqwest::Client::new(),
        }
    }

    /// Default: Ollama on localhost
    pub fn ollama(model: &str) -> Self {
        Self::new("http://localhost:11434", model)
    }

    /// Send a chat completion request and return the response text
    pub async fn chat(
        &self,
        system_prompt: &str,
        user_prompt: &str,
    ) -> Result<String, String> {
        let messages = vec![
            ChatMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessage {
                role: "user".to_string(),
                content: user_prompt.to_string(),
            },
        ];

        self.chat_messages(&messages).await
    }

    /// Send messages and return the response text
    pub async fn chat_messages(&self, messages: &[ChatMessage]) -> Result<String, String> {
        let url = format!("{}/v1/chat/completions", self.base_url);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: messages.to_vec(),
            temperature: 0.3,
            stream: false,
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
            .first()
            .map(|c| c.message.content.clone())
            .ok_or("No response from LLM".to_string())
    }

    /// Check if the LLM server is reachable
    pub async fn health_check(&self) -> Result<bool, String> {
        let url = format!("{}/v1/models", self.base_url);
        match self.http.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}
