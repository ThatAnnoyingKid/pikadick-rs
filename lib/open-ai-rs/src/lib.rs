mod request;
mod response;

use self::request::{
    ChatCompletionRequest,
    CompletionRequest,
};
pub use self::response::{
    ChatCompletionResponse,
    ChatCompletionResponseChoice,
    CompletionResponse,
    CompletionResponseChoice,
};
use std::sync::Arc;

/// A chat completion request message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    /// The role
    pub role: Box<str>,

    /// The content
    pub content: Box<str>,
}

/// The library error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Reqwest HTTP Error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
}

/// An open ai client
#[derive(Debug, Clone)]
pub struct Client {
    /// The inner http client
    pub client: reqwest::Client,

    /// The api key
    key: Arc<str>,
}

impl Client {
    /// Make a new client
    pub fn new(key: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            key: key.into(),
        }
    }

    /// Perform a completion.
    pub async fn completion(
        &self,
        model: &str,
        max_tokens: u16,
        prompt: &str,
    ) -> Result<CompletionResponse, Error> {
        Ok(self
            .client
            .post("https://api.openai.com/v1/completions")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.key),
            )
            .json(&CompletionRequest {
                model,
                max_tokens,
                prompt,
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }

    /// Perform a chat completion.
    pub async fn chat_completion(
        &self,
        model: &str,
        messages: &[ChatMessage],
    ) -> Result<ChatCompletionResponse, Error> {
        Ok(self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", self.key),
            )
            .json(&ChatCompletionRequest {
                model: model.into(),
                messages: messages.into(),
            })
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const KEY: &str = include_str!("../key.txt");

    #[ignore]
    #[tokio::test]
    async fn it_works() {
        let client = Client::new(KEY);
        let response = client
            .chat_completion(
                "gpt-3.5-turbo",
                &[ChatMessage {
                    role: "user".into(),
                    content: "Hello! How are you today?".into(),
                }],
            )
            .await
            .expect("failed to get response");
        dbg!(&response);
    }

    #[test]
    fn parse_completion_response() {
        let text = include_str!("../test_data/completion_response.json");
        let response: CompletionResponse = serde_json::from_str(text).expect("failed to parse");
        dbg!(&response);
    }

    #[test]
    fn parse_chat_completion_response() {
        let text = include_str!("../test_data/chat_completion_response.json");
        let response: ChatCompletionResponse = serde_json::from_str(text).expect("failed to parse");
        dbg!(&response);
    }
}
