use crate::ChatMessage;

#[derive(Debug, serde::Serialize)]
pub struct CompletionRequest<'a, 'b> {
    /// The model
    pub model: &'a str,

    /// The prompt
    pub prompt: &'b str,

    /// The max number of tokens to return
    pub max_tokens: u16,
}

/// A chat completion request
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatCompletionRequest {
    /// The model
    pub model: Box<str>,

    /// The messages in the conversation
    pub messages: Vec<ChatMessage>,

    /// The max number of tokens to return
    pub max_tokens: Option<u16>,
}
