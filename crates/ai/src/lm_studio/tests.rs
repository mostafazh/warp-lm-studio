//! Unit and integration tests for the LM Studio provider module.

use std::sync::{Mutex, OnceLock};

use super::*;
use crate::api_keys::LmStudioApiConfig;
use crate::lm_studio::client::LmStudioClient;
use crate::lm_studio::types::{ChatMessage, ChatRole};

/// Global mutex to serialize tests that modify environment variables.
/// Env vars are process-global, so parallel test threads can interfere.
static ENV_MUTEX: Mutex<()> = Mutex::new(());

/// Install the rustls crypto provider once per test process.
///
/// The workspace uses the `rustls-tls-native-roots-no-provider` reqwest
/// feature, which means the caller must register a provider before any TLS
/// connections can be made. In the production app this is done early in
/// `main`; in tests we call it lazily on first use.
static CRYPTO_PROVIDER: OnceLock<()> = OnceLock::new();

fn init_crypto_provider() {
    CRYPTO_PROVIDER.get_or_init(|| {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    });
}

/// Build a `reqwest::Client` suitable for use in tests.
fn test_reqwest_client() -> reqwest::Client {
    init_crypto_provider();
    reqwest::Client::builder()
        .tls_built_in_native_certs(false)
        .tls_built_in_root_certs(false)
        .no_proxy()
        .build()
        .expect("Failed to build test reqwest client; check that the rustls \
                 crypto provider was initialized successfully")
}

/// Convenience: create an [`LmStudioClient`] using the test-safe HTTP client.
fn test_client(config: LmStudioApiConfig) -> LmStudioClient {
    LmStudioClient::with_reqwest_client(config, test_reqwest_client())
}

// ── Config resolution ──────────────────────────────────────────────────────────

#[test]
fn resolve_config_uses_defaults_when_nothing_configured() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard1 = env_remove_guard(ENV_BASE_URL);
    let _guard2 = env_remove_guard(ENV_API_KEY);
    let _guard3 = env_remove_guard(ENV_MODEL);

    let config = resolve_config(None);
    assert_eq!(config.base_url, None);
    assert_eq!(config.api_key, None);
    assert_eq!(config.model, None);
}

#[test]
fn resolve_config_prefers_env_vars_over_persisted() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard1 = temp_env(ENV_BASE_URL, "http://custom:5678/v1");
    let _guard2 = temp_env(ENV_API_KEY, "env-key");
    let _guard3 = temp_env(ENV_MODEL, "my-model");

    let persisted = LmStudioApiConfig {
        base_url: Some("http://persisted:1234/v1".into()),
        api_key: Some("persisted-key".into()),
        model: Some("persisted-model".into()),
    };

    let config = resolve_config(Some(&persisted));

    assert_eq!(config.base_url.as_deref(), Some("http://custom:5678/v1"));
    assert_eq!(config.api_key.as_deref(), Some("env-key"));
    assert_eq!(config.model.as_deref(), Some("my-model"));
}

#[test]
fn resolve_config_falls_back_to_persisted_when_env_vars_absent() {
    let _lock = ENV_MUTEX.lock().unwrap();
    let _guard1 = env_remove_guard(ENV_BASE_URL);
    let _guard2 = env_remove_guard(ENV_API_KEY);
    let _guard3 = env_remove_guard(ENV_MODEL);

    let persisted = LmStudioApiConfig {
        base_url: Some("http://persisted:9999/v1".into()),
        api_key: None,
        model: Some("gemma-2-2b-it".into()),
    };

    let config = resolve_config(Some(&persisted));

    assert_eq!(config.base_url.as_deref(), Some("http://persisted:9999/v1"));
    assert_eq!(config.api_key, None);
    assert_eq!(config.model.as_deref(), Some("gemma-2-2b-it"));
}

// ── Client URL construction ────────────────────────────────────────────────────

#[test]
fn client_uses_default_base_url_when_none_configured() {
    let client = test_client(LmStudioApiConfig::default());
    assert_eq!(
        client.chat_completions_url(),
        "http://localhost:1234/v1/chat/completions"
    );
}

#[test]
fn client_strips_trailing_slash_from_base_url() {
    let client = test_client(LmStudioApiConfig {
        base_url: Some("http://localhost:1234/v1/".into()),
        ..Default::default()
    });
    assert_eq!(
        client.chat_completions_url(),
        "http://localhost:1234/v1/chat/completions"
    );
}

#[test]
fn client_uses_custom_port() {
    let client = test_client(LmStudioApiConfig {
        base_url: Some("http://localhost:9090/v1".into()),
        ..Default::default()
    });
    assert_eq!(
        client.chat_completions_url(),
        "http://localhost:9090/v1/chat/completions"
    );
}

// ── Request headers ────────────────────────────────────────────────────────────

#[test]
fn no_auth_header_when_api_key_not_set() {
    let client = test_client(LmStudioApiConfig {
        api_key: None,
        ..Default::default()
    });
    let headers = client.request_headers();
    assert!(
        !headers.contains_key(reqwest::header::AUTHORIZATION),
        "Authorization header should be absent when no API key is configured"
    );
}

#[test]
fn no_auth_header_when_api_key_is_empty_string() {
    let client = test_client(LmStudioApiConfig {
        api_key: Some(String::new()),
        ..Default::default()
    });
    let headers = client.request_headers();
    assert!(
        !headers.contains_key(reqwest::header::AUTHORIZATION),
        "Authorization header should be absent when API key is empty"
    );
}

#[test]
fn auth_header_set_when_api_key_provided() {
    let client = test_client(LmStudioApiConfig {
        api_key: Some("lm-studio".into()),
        ..Default::default()
    });
    let headers = client.request_headers();
    let auth = headers
        .get(reqwest::header::AUTHORIZATION)
        .expect("Authorization header should be present");
    assert_eq!(auth.to_str().unwrap(), "Bearer lm-studio");
}

#[test]
fn content_type_header_always_set() {
    let client = test_client(LmStudioApiConfig::default());
    let headers = client.request_headers();
    let ct = headers
        .get(reqwest::header::CONTENT_TYPE)
        .expect("Content-Type should be present");
    assert_eq!(ct.to_str().unwrap(), "application/json");
}

// ── Request building ───────────────────────────────────────────────────────────

#[test]
fn build_request_uses_configured_model() {
    let client = test_client(LmStudioApiConfig {
        model: Some("gemma-2-9b-it".into()),
        ..Default::default()
    });
    let messages = vec![ChatMessage::user("Hello")];
    let req = client.build_request(messages.clone(), false);

    assert_eq!(req.model, "gemma-2-9b-it");
    assert_eq!(req.messages, messages);
    assert!(!req.stream);
}

#[test]
fn build_request_sets_stream_flag() {
    let client = test_client(LmStudioApiConfig::default());
    let req = client.build_request(vec![ChatMessage::user("Hi")], true);
    assert!(req.stream);
}

#[test]
fn build_request_empty_model_when_not_configured() {
    let client = test_client(LmStudioApiConfig::default());
    let req = client.build_request(vec![], false);
    assert_eq!(req.model, "", "model should be empty string when not configured");
}

// ── Chat message helpers ───────────────────────────────────────────────────────

#[test]
fn chat_message_constructors() {
    let sys = ChatMessage::system("You are a helpful assistant.");
    assert_eq!(sys.role, ChatRole::System);

    let user = ChatMessage::user("Hello");
    assert_eq!(user.role, ChatRole::User);

    let asst = ChatMessage::assistant("Hi there");
    assert_eq!(asst.role, ChatRole::Assistant);
}

// ── Response parsing ───────────────────────────────────────────────────────────

#[test]
fn parse_chat_completion_response() {
    let json = r#"{
        "id": "chatcmpl-123",
        "object": "chat.completion",
        "created": 1700000000,
        "model": "gemma-2-2b-it",
        "choices": [
            {
                "index": 0,
                "message": { "role": "assistant", "content": "Hello! How can I help?" },
                "finish_reason": "stop"
            }
        ],
        "usage": { "prompt_tokens": 10, "completion_tokens": 8, "total_tokens": 18 }
    }"#;

    let response: types::ChatCompletionResponse =
        serde_json::from_str(json).expect("should parse response");

    assert_eq!(response.id, "chatcmpl-123");
    assert_eq!(response.model, "gemma-2-2b-it");
    assert_eq!(
        response.first_content(),
        Some("Hello! How can I help?")
    );
}

#[test]
fn parse_streaming_chunk() {
    let json = r#"{
        "id": "chatcmpl-456",
        "object": "chat.completion.chunk",
        "created": 1700000001,
        "model": "gemma-2-2b-it",
        "choices": [
            {
                "index": 0,
                "delta": { "content": " world" },
                "finish_reason": null
            }
        ]
    }"#;

    let chunk: types::ChatCompletionChunk =
        serde_json::from_str(json).expect("should parse chunk");

    assert_eq!(chunk.delta_content(), Some(" world"));
    assert!(!chunk.is_finished());
}

#[test]
fn parse_final_streaming_chunk() {
    let json = r#"{
        "id": "chatcmpl-789",
        "object": "chat.completion.chunk",
        "created": 1700000002,
        "model": "gemma-2-2b-it",
        "choices": [
            {
                "index": 0,
                "delta": {},
                "finish_reason": "stop"
            }
        ]
    }"#;

    let chunk: types::ChatCompletionChunk =
        serde_json::from_str(json).expect("should parse final chunk");

    assert!(chunk.is_finished());
    assert_eq!(chunk.delta_content(), None);
}

// ── ApiKeys serialization ──────────────────────────────────────────────────────

#[test]
fn api_keys_serializes_lm_studio_config() {
    let keys = crate::api_keys::ApiKeys {
        lm_studio: Some(LmStudioApiConfig {
            base_url: Some("http://localhost:1234/v1".into()),
            api_key: None,
            model: Some("gemma-2-2b-it".into()),
        }),
        ..Default::default()
    };

    let serialized = serde_json::to_string(&keys).unwrap();
    let roundtripped: crate::api_keys::ApiKeys =
        serde_json::from_str(&serialized).unwrap();

    assert_eq!(keys, roundtripped);
}

#[test]
fn api_keys_without_lm_studio_deserializes_as_none() {
    // Old format (no lm_studio field) should deserialize with lm_studio = None.
    let json = r#"{"google":null,"anthropic":null,"openai":null,"open_router":null}"#;
    let keys: crate::api_keys::ApiKeys = serde_json::from_str(json).unwrap();
    assert_eq!(keys.lm_studio, None);
}

#[test]
fn api_keys_has_any_key_includes_lm_studio() {
    let mut keys = crate::api_keys::ApiKeys::default();
    assert!(!keys.has_any_key());

    keys.lm_studio = Some(LmStudioApiConfig::default());
    assert!(keys.has_any_key());
}

// ── HTTP integration test (mockito) ───────────────────────────────────────────
//
// These tests start a local mockito server and verify that the client sends the
// right request and correctly parses the response.  They require a multi-thread
// Tokio runtime.

#[tokio::test]
async fn complete_sends_correct_request_and_parses_response() {
    let mut server = mockito::Server::new_async().await;

    let response_body = serde_json::json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "created": 1700000000u64,
        "model": "gemma-2-2b-it",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "Mocked response" },
            "finish_reason": "stop"
        }],
        "usage": { "prompt_tokens": 5, "completion_tokens": 3, "total_tokens": 8 }
    });

    let mock = server
        .mock("POST", "/v1/chat/completions")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body.to_string())
        .create_async()
        .await;

    let config = LmStudioApiConfig {
        base_url: Some(format!("{}/v1", server.url())),
        api_key: None,
        model: Some("gemma-2-2b-it".into()),
    };

    let client = LmStudioClient::with_reqwest_client(config, test_reqwest_client());

    let request = client.build_request(vec![ChatMessage::user("Test")], false);
    let response = client.complete(request).await.expect("request should succeed");

    assert_eq!(response.first_content(), Some("Mocked response"));
    mock.assert_async().await;
}

#[tokio::test]
async fn complete_returns_server_unreachable_error_when_server_down() {
    // Point to a port that's definitely not listening.
    let config = LmStudioApiConfig {
        base_url: Some("http://127.0.0.1:19999/v1".into()),
        ..Default::default()
    };
    let client = LmStudioClient::with_reqwest_client(config, test_reqwest_client());
    let request = client.build_request(vec![ChatMessage::user("Hi")], false);
    let err = client.complete(request).await.unwrap_err();

    assert!(
        matches!(err, types::LmStudioError::ServerUnreachable { .. }),
        "expected ServerUnreachable, got: {err:?}"
    );
    // The error message should contain a helpful hint.
    let msg = err.to_string();
    assert!(
        msg.contains("LM Studio"),
        "error message should mention LM Studio: {msg}"
    );
}

#[tokio::test]
async fn complete_with_api_key_sends_authorization_header() {
    let mut server = mockito::Server::new_async().await;

    let response_body = serde_json::json!({
        "id": "chatcmpl-key",
        "object": "chat.completion",
        "created": 1700000000u64,
        "model": "gemma-2-2b-it",
        "choices": [{
            "index": 0,
            "message": { "role": "assistant", "content": "ok" },
            "finish_reason": "stop"
        }]
    });

    let mock = server
        .mock("POST", "/v1/chat/completions")
        .match_header("authorization", "Bearer test-key")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(response_body.to_string())
        .create_async()
        .await;

    let config = LmStudioApiConfig {
        base_url: Some(format!("{}/v1", server.url())),
        api_key: Some("test-key".into()),
        model: Some("gemma-2-2b-it".into()),
    };

    let client = LmStudioClient::with_reqwest_client(config, test_reqwest_client());
    let request = client.build_request(vec![ChatMessage::user("Hi")], false);
    client.complete(request).await.expect("should succeed");

    mock.assert_async().await;
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Remove an environment variable for the duration of a test; restore on drop.
struct EnvRemoveGuard {
    key: &'static str,
    old: Option<String>,
}

impl Drop for EnvRemoveGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn env_remove_guard(key: &'static str) -> EnvRemoveGuard {
    let old = std::env::var(key).ok();
    std::env::remove_var(key);
    EnvRemoveGuard { key, old }
}

/// Temporarily set an environment variable; restore on drop.
struct TempEnvGuard {
    key: &'static str,
    old: Option<String>,
}

impl Drop for TempEnvGuard {
    fn drop(&mut self) {
        match &self.old {
            Some(v) => std::env::set_var(self.key, v),
            None => std::env::remove_var(self.key),
        }
    }
}

fn temp_env(key: &'static str, value: &str) -> TempEnvGuard {
    let old = std::env::var(key).ok();
    std::env::set_var(key, value);
    TempEnvGuard { key, old }
}
