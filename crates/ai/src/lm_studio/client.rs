//! HTTP client for LM Studio's OpenAI-compatible API.
//!
//! [`LmStudioClient`] is a thin async client that constructs the correct URL
//! and `Authorization` header for LM Studio, then delegates the actual HTTP
//! work to a [`reqwest::Client`].
//!
//! Both non-streaming (full JSON) and streaming (SSE via `reqwest-eventsource`)
//! requests are supported.

use anyhow;
use futures::Stream;
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE, HeaderMap, HeaderValue};
use reqwest_eventsource::{Event, EventSource};
use serde_json;

use crate::api_keys::LmStudioApiConfig;
use crate::lm_studio::{DEFAULT_BASE_URL, types::*};

/// HTTP client for LM Studio's OpenAI-compatible API.
///
/// # Example
///
/// ```no_run
/// use ai::lm_studio::{LmStudioClient, ChatMessage, ChatCompletionRequest};
/// use ai::api_keys::LmStudioApiConfig;
///
/// # tokio_test::block_on(async {
/// let config = LmStudioApiConfig {
///     base_url: None,   // defaults to http://localhost:1234/v1
///     api_key: None,    // no key required by LM Studio
///     model: Some("gemma-2-2b-it".into()),
/// };
/// let client = LmStudioClient::new(config);
///
/// let request = client.build_request(
///     vec![ChatMessage::user("Hello, Gemma!")],
///     false, // not streaming
/// );
/// // let response = client.complete(request).await.unwrap();
/// # });
/// ```
pub struct LmStudioClient {
    /// The resolved base URL (e.g. `http://localhost:1234/v1`).
    base_url: String,
    /// Optional API key. Sent as `Authorization: Bearer <key>` when present.
    api_key: Option<String>,
    /// Default model identifier for requests.
    model: String,
    inner: reqwest::Client,
}

impl LmStudioClient {
    /// Construct a new client from a [`LmStudioApiConfig`].
    ///
    /// The `base_url`, `api_key`, and `model` fields of the config are used
    /// directly; sensible defaults are applied when fields are `None`.
    pub fn new(config: LmStudioApiConfig) -> Self {
        Self::with_reqwest_client(config, reqwest::Client::new())
    }

    /// Construct a new client using a caller-supplied [`reqwest::Client`].
    ///
    /// Use this in tests to inject a mock client built against a
    /// `mockito::Server`.
    pub fn with_reqwest_client(config: LmStudioApiConfig, client: reqwest::Client) -> Self {
        let base_url = config
            .base_url
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned());
        // Strip trailing slash so we can append "/chat/completions" cleanly.
        let base_url = base_url.trim_end_matches('/').to_owned();
        let model = config.model.unwrap_or_default();

        Self {
            base_url,
            api_key: config.api_key,
            model,
            inner: client,
        }
    }

    /// The full URL for the `/v1/chat/completions` endpoint.
    pub fn chat_completions_url(&self) -> String {
        format!("{}/chat/completions", self.base_url)
    }

    /// Build the `Authorization` header value, if an API key is configured.
    ///
    /// LM Studio does not require a real API key; when no key is set, the
    /// header is omitted rather than sending an empty `Bearer ` token.
    pub fn auth_header_value(&self) -> Option<HeaderValue> {
        self.api_key.as_deref().and_then(|key| {
            if key.is_empty() {
                None
            } else {
                let value = format!("Bearer {key}");
                HeaderValue::from_str(&value).ok()
            }
        })
    }

    /// Build the full set of request headers.
    pub fn request_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        if let Some(auth) = self.auth_header_value() {
            headers.insert(AUTHORIZATION, auth);
        }
        headers
    }

    /// Construct a [`ChatCompletionRequest`] from the given messages.
    ///
    /// Uses the model configured on this client (falling back to an empty
    /// string, which causes LM Studio to use the currently loaded model).
    pub fn build_request(
        &self,
        messages: Vec<ChatMessage>,
        stream: bool,
    ) -> ChatCompletionRequest {
        ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            stream,
            temperature: None,
            max_tokens: None,
        }
    }

    /// Send a non-streaming chat completion request to LM Studio.
    ///
    /// Returns the full [`ChatCompletionResponse`] or an [`LmStudioError`].
    pub async fn complete(
        &self,
        request: ChatCompletionRequest,
    ) -> Result<ChatCompletionResponse, LmStudioError> {
        let url = self.chat_completions_url();
        let body = serde_json::to_string(&request)?;

        let response = self
            .inner
            .post(&url)
            .headers(self.request_headers())
            .body(body)
            .send()
            .await
            .map_err(|e| LmStudioError::ServerUnreachable {
                url: url.clone(),
                source: anyhow::Error::new(e).context(
                    "Failed to connect to LM Studio. \
                     Ensure LM Studio is open and the local server is running \
                     (Server → Start Server).",
                ),
            })?;

        let status = response.status();
        if !status.is_success() {
            let body_text = response
                .text()
                .await
                .unwrap_or_else(|_| "<could not read body>".to_owned());
            return Err(LmStudioError::HttpError {
                status: status.as_u16(),
                body: body_text,
            });
        }

        let response_body = response
            .text()
            .await
            .map_err(|e| {
                LmStudioError::Other(
                    anyhow::Error::new(e).context("Failed to read LM Studio response body"),
                )
            })?;

        let parsed: ChatCompletionResponse = serde_json::from_str(&response_body)?;
        Ok(parsed)
    }

    /// Send a streaming chat completion request and return a stream of
    /// [`ChatCompletionChunk`]s.
    ///
    /// Tokens are delivered incrementally via Server-Sent Events (SSE).
    /// The stream ends when LM Studio sends a `[DONE]` event or sets a
    /// `finish_reason` on the last chunk.
    pub fn complete_stream(
        &self,
        request: ChatCompletionRequest,
    ) -> impl Stream<Item = Result<ChatCompletionChunk, LmStudioError>> {
        let url = self.chat_completions_url();
        let headers = self.request_headers();

        // Build the reqwest RequestBuilder and attach the SSE adapter.
        let builder = self
            .inner
            .post(&url)
            .headers(headers)
            .json(&request);

        let event_source = EventSource::new(builder)
            .expect("Failed to create EventSource from reqwest RequestBuilder; \
                     ensure the request URL and headers are valid");

        // Wrap the EventSource stream, translating each event into a chunk.
        futures::stream::unfold(event_source, |mut es| async move {
            loop {
                match futures::StreamExt::next(&mut es).await {
                    None => return None,
                    Some(Ok(Event::Open)) => continue,
                    Some(Ok(Event::Message(msg))) => {
                        if msg.data == "[DONE]" {
                            return None;
                        }
                        let chunk: Result<ChatCompletionChunk, LmStudioError> =
                            serde_json::from_str(&msg.data).map_err(LmStudioError::Json);
                        return Some((chunk, es));
                    }
                    Some(Err(e)) => {
                        let err = LmStudioError::Other(anyhow::anyhow!(
                            "SSE stream error from LM Studio: {e}"
                        ));
                        return Some((Err(err), es));
                    }
                }
            }
        })
    }
}
