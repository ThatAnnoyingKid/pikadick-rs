use crate::ChatMessage;

/// The response for a completion request
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CompletionResponse {
    /// The id of the response
    pub id: Box<str>,

    /// ?
    pub object: Box<str>,

    /// ?
    pub created: u64,

    /// The used model
    pub model: Box<str>,

    /// ?
    pub choices: Vec<CompletionResponseChoice>,
    // pub usage:
}

/// A completion response choice
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CompletionResponseChoice {
    /// The text response?
    pub text: Box<str>,
    // pub index:
}

/// A chat completion response
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatCompletionResponse {
    /// id of completion?
    pub id: Box<str>,

    /// ?
    pub object: Box<str>,

    /// ?
    pub created: u64,

    /// ?
    pub choices: Vec<ChatCompletionResponseChoice>,
    //pub usage
}

/// A chat completion response choice
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ChatCompletionResponseChoice {
    // pub index
    /// The message
    pub message: ChatMessage,
    // finish_reason
}
