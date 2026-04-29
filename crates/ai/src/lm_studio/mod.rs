//! LM Studio local inference provider.
//!
//! [LM Studio](https://lmstudio.ai) can serve local models (e.g. Gemma) via an
//! OpenAI-compatible HTTP API. This module provides configuration helpers and
//! request/response types for talking to that API directly from the Warp client,
//! without routing through Warp's own backend server.
//!
//! # Quick start
//!
//! 1. Open LM Studio and load the model you want (e.g. `gemma-2-2b-it`).
//! 2. Go to **Server → Start Server** and ensure "OpenAI Compatible Server"
//!    is enabled. The default port is **1234**.
//! 3. Export the environment variables (or configure them via Warp's settings):
//!
//! ```sh
//! export WARP_LM_STUDIO_BASE_URL="http://localhost:1234/v1"   # default
//! export WARP_LM_STUDIO_MODEL="gemma-2-2b-it"
//! # API key is optional; LM Studio does not require a real key.
//! # export WARP_LM_STUDIO_API_KEY="lm-studio"
//! ```
//!
//! # Architecture
//!
//! Warp's default inference path routes requests through the Warp backend
//! server. LM Studio, however, runs **locally** on the user's machine, so
//! requests must bypass the backend and go directly to `localhost:1234`.
//! This module provides the building blocks (config, request types, URL/header
//! construction) for that direct path. Wiring it into the agent conversation
//! flow is handled in the `app` crate.

pub mod client;
pub mod types;

#[cfg(test)]
mod tests;

pub use client::LmStudioClient;
pub use types::{
    ChatCompletionChunk, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ChatRole,
    LmStudioError,
};

use crate::api_keys::LmStudioApiConfig;

/// Default base URL for LM Studio's OpenAI-compatible HTTP server.
pub const DEFAULT_BASE_URL: &str = "http://localhost:1234/v1";

/// Environment variable that overrides the LM Studio base URL.
pub const ENV_BASE_URL: &str = "WARP_LM_STUDIO_BASE_URL";

/// Environment variable that overrides the LM Studio API key.
/// LM Studio does not require a real API key; a placeholder value such as
/// `"lm-studio"` is sufficient if your HTTP client insists on a non-empty
/// `Authorization` header.
pub const ENV_API_KEY: &str = "WARP_LM_STUDIO_API_KEY";

/// Environment variable that sets the model identifier sent to LM Studio
/// (e.g. `"gemma-2-2b-it"`, `"gemma-2-9b-it"`).
pub const ENV_MODEL: &str = "WARP_LM_STUDIO_MODEL";

/// Build an [`LmStudioApiConfig`] by merging persisted settings with environment
/// variable overrides.
///
/// Priority order (highest first):
/// 1. `WARP_LM_STUDIO_*` environment variables
/// 2. Persisted [`LmStudioApiConfig`] (from Warp settings / secure storage)
/// 3. Built-in defaults
pub fn resolve_config(persisted: Option<&LmStudioApiConfig>) -> LmStudioApiConfig {
    let base_url = std::env::var(ENV_BASE_URL)
        .ok()
        .or_else(|| persisted.and_then(|c| c.base_url.clone()));

    let api_key = std::env::var(ENV_API_KEY)
        .ok()
        .or_else(|| persisted.and_then(|c| c.api_key.clone()));

    let model = std::env::var(ENV_MODEL)
        .ok()
        .or_else(|| persisted.and_then(|c| c.model.clone()));

    LmStudioApiConfig {
        base_url,
        api_key,
        model,
    }
}
