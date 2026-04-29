//! OpenAI-compatible request/response types used by LM Studio.
//!
//! These types mirror the OpenAI Chat Completions API format that LM Studio
//! exposes at `POST /v1/chat/completions`. Both standard (non-streaming) and
//! streaming (SSE) response formats are represented here.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Role of a participant in a chat conversation.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChatRole {
    System,
    User,
    Assistant,
}

/// A single message in a chat conversation.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    /// Role of the sender.
    pub role: ChatRole,
    /// Text content of the message.
    pub content: String,
}

impl ChatMessage {
    /// Create a new system message.
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::System,
            content: content.into(),
        }
    }

    /// Create a new user message.
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::User,
            content: content.into(),
        }
    }

    /// Create a new assistant message.
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: ChatRole::Assistant,
            content: content.into(),
        }
    }
}

/// Request body for `POST /v1/chat/completions`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChatCompletionRequest {
    /// The model to use (e.g. `"gemma-2-2b-it"`).
    /// When empty, LM Studio uses the currently loaded model.
    pub model: String,
    /// Conversation messages.
    pub messages: Vec<ChatMessage>,
    /// Whether to stream tokens as Server-Sent Events.
    #[serde(default)]
    pub stream: bool,
    /// Optional sampling temperature (0.0 – 2.0). `None` uses the server default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Optional maximum tokens to generate. `None` uses the server default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

/// Token usage statistics returned in a non-streaming completion response.
#[derive(Clone, Debug, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

/// A single choice in a non-streaming completion response.
#[derive(Clone, Debug, Deserialize)]
pub struct CompletionChoice {
    pub index: u32,
    pub message: ChatMessage,
    /// Reason the model stopped generating (e.g. `"stop"`, `"length"`).
    pub finish_reason: Option<String>,
}

/// Full (non-streaming) response body from `POST /v1/chat/completions`.
#[derive(Clone, Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<CompletionChoice>,
    #[serde(default)]
    pub usage: Option<TokenUsage>,
}

impl ChatCompletionResponse {
    /// Return the first choice's content, if present.
    pub fn first_content(&self) -> Option<&str> {
        self.choices
            .first()
            .map(|c| c.message.content.as_str())
    }
}

/// Delta content inside a streaming chunk choice.
#[derive(Clone, Debug, Deserialize)]
pub struct ChunkDelta {
    #[serde(default)]
    pub role: Option<ChatRole>,
    #[serde(default)]
    pub content: Option<String>,
}

/// A single choice inside a streaming `ChatCompletionChunk`.
#[derive(Clone, Debug, Deserialize)]
pub struct ChunkChoice {
    pub index: u32,
    pub delta: ChunkDelta,
    pub finish_reason: Option<String>,
}

/// Server-Sent Event payload for streaming `POST /v1/chat/completions`.
#[derive(Clone, Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<ChunkChoice>,
}

impl ChatCompletionChunk {
    /// Collect the incremental content from the first choice's delta.
    pub fn delta_content(&self) -> Option<&str> {
        self.choices
            .first()
            .and_then(|c| c.delta.content.as_deref())
    }

    /// Returns `true` if this is the final chunk (finish_reason is set).
    pub fn is_finished(&self) -> bool {
        self.choices
            .first()
            .and_then(|c| c.finish_reason.as_deref())
            .is_some()
    }
}

/// Errors that can occur when communicating with the LM Studio server.
#[derive(Debug, Error)]
pub enum LmStudioError {
    /// The LM Studio server could not be reached (e.g. it is not running).
    #[error(
        "Cannot reach the LM Studio server at {url}. \
         Make sure LM Studio is running and the server is started \
         (Server → Start Server in LM Studio). \
         Source: {source}"
    )]
    ServerUnreachable {
        url: String,
        #[source]
        source: anyhow::Error,
    },

    /// The server returned a non-2xx HTTP status code.
    #[error("LM Studio returned HTTP {status}: {body}")]
    HttpError { status: u16, body: String },

    /// Failed to serialize the request or deserialize the response.
    #[error("JSON error communicating with LM Studio: {0}")]
    Json(#[from] serde_json::Error),

    /// Any other error.
    #[error("LM Studio error: {0}")]
    Other(#[from] anyhow::Error),
}
