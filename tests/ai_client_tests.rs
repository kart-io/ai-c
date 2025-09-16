//! Unit tests for AIClient
//!
//! Tests the AI HTTP client functionality for different providers

use ai_c::ai::{
    client::{AIClient, AIClientConfig, AIProvider, AIRequest, HttpAIClient},
};
use std::time::Duration;
use tokio;

/// Create a test AIClient with mock configuration
fn create_test_client(provider: AIProvider) -> HttpAIClient {
    let config = AIClientConfig {
        provider: provider.clone(),
        base_url: provider.default_base_url().to_string(),
        api_key: Some("test-api-key".to_string()),
        model: provider.default_model().to_string(),
        timeout: Duration::from_secs(5),
        max_retries: 1,
        retry_delay: Duration::from_millis(100),
        enable_logging: false,
    };

    HttpAIClient::new(config).expect("Failed to create test client")
}


#[tokio::test]
async fn test_ai_client_creation() {
    let client = create_test_client(AIProvider::OpenAI);
    assert_eq!(client.config().provider, AIProvider::OpenAI);
    assert_eq!(client.config().max_retries, 1);
    assert_eq!(client.config().timeout, Duration::from_secs(5));
}

#[tokio::test]
async fn test_different_providers() {
    let providers = vec![
        AIProvider::OpenAI,
        AIProvider::Anthropic,
        AIProvider::Ollama,
        AIProvider::Custom("custom-provider".to_string()),
    ];

    for provider in providers {
        let client = create_test_client(provider.clone());
        assert_eq!(client.config().provider, provider);

        // Test provider-specific configurations
        match provider {
            AIProvider::OpenAI => {
                assert_eq!(client.config().base_url, "https://api.openai.com/v1");
                assert_eq!(client.config().model, "gpt-3.5-turbo");
            }
            AIProvider::Anthropic => {
                assert_eq!(client.config().base_url, "https://api.anthropic.com/v1");
                assert_eq!(client.config().model, "claude-3-haiku-20240307");
            }
            AIProvider::Ollama => {
                assert_eq!(client.config().base_url, "http://localhost:11434/api");
                assert_eq!(client.config().model, "llama2");
            }
            AIProvider::Custom(_) => {
                // Custom provider uses empty defaults
            }
        }
    }
}

#[tokio::test]
async fn test_ai_request_builder() {
    let request = AIRequest::new("Test prompt".to_string())
        .with_model("custom-model".to_string())
        .with_max_tokens(200)
        .with_temperature(0.8)
        .with_parameter("custom_param".to_string(), serde_json::json!("value"));

    assert_eq!(request.prompt, "Test prompt");
    assert_eq!(request.model, Some("custom-model".to_string()));
    assert_eq!(request.max_tokens, Some(200));
    assert_eq!(request.temperature, Some(0.8));
    assert_eq!(request.parameters.len(), 1);
    assert_eq!(request.parameters.get("custom_param"), Some(&serde_json::json!("value")));
}

#[tokio::test]
async fn test_temperature_clamping() {
    let request1 = AIRequest::new("Test".to_string()).with_temperature(-0.5);
    assert_eq!(request1.temperature, Some(0.0));

    let request2 = AIRequest::new("Test".to_string()).with_temperature(1.5);
    assert_eq!(request2.temperature, Some(1.0));

    let request3 = AIRequest::new("Test".to_string()).with_temperature(0.5);
    assert_eq!(request3.temperature, Some(0.5));
}

#[tokio::test]
async fn test_client_config_validation() {
    // Test invalid API key handling
    let config = AIClientConfig {
        provider: AIProvider::OpenAI,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("".to_string()), // Empty API key
        model: "gpt-3.5-turbo".to_string(),
        timeout: Duration::from_secs(30),
        max_retries: 3,
        retry_delay: Duration::from_secs(1),
        enable_logging: true,
    };

    let result = HttpAIClient::new(config);
    assert!(result.is_ok(), "Should handle empty API key");
}

#[tokio::test]
async fn test_client_endpoints() {
    let openai_client = create_test_client(AIProvider::OpenAI);
    let anthropic_client = create_test_client(AIProvider::Anthropic);
    let ollama_client = create_test_client(AIProvider::Ollama);

    // We can't test private methods directly, but we can verify the client creation
    // which uses the endpoint logic internally
    assert!(openai_client.config().base_url.contains("openai.com"));
    assert!(anthropic_client.config().base_url.contains("anthropic.com"));
    assert!(ollama_client.config().base_url.contains("localhost"));
}

#[tokio::test]
async fn test_client_statistics() {
    let client = create_test_client(AIProvider::OpenAI);

    // Test public statistics calculation methods
    assert_eq!(client.success_rate(), 0.0);
    assert_eq!(client.average_response_time(), Duration::ZERO);

    // Note: We can't test actual statistics updates since send_request
    // would require a real API endpoint, but we can test the calculation logic
}

#[tokio::test]
async fn test_client_with_no_api_key() {
    let config = AIClientConfig {
        provider: AIProvider::OpenAI,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: None,
        model: "gpt-3.5-turbo".to_string(),
        timeout: Duration::from_secs(30),
        max_retries: 3,
        retry_delay: Duration::from_secs(1),
        enable_logging: true,
    };

    let result = HttpAIClient::new(config);
    assert!(result.is_ok(), "Should create client without API key");
}

#[tokio::test]
async fn test_provider_default_models() {
    assert_eq!(AIProvider::OpenAI.default_model(), "gpt-3.5-turbo");
    assert_eq!(AIProvider::Anthropic.default_model(), "claude-3-haiku-20240307");
    assert_eq!(AIProvider::Ollama.default_model(), "llama2");
    assert_eq!(AIProvider::Custom("test".to_string()).default_model(), "");
}

#[tokio::test]
async fn test_provider_default_urls() {
    assert_eq!(AIProvider::OpenAI.default_base_url(), "https://api.openai.com/v1");
    assert_eq!(AIProvider::Anthropic.default_base_url(), "https://api.anthropic.com/v1");
    assert_eq!(AIProvider::Ollama.default_base_url(), "http://localhost:11434/api");
    assert_eq!(AIProvider::Custom("test".to_string()).default_base_url(), "");
}

#[tokio::test]
async fn test_ai_client_config_default() {
    let config = AIClientConfig::default();

    assert_eq!(config.provider, AIProvider::OpenAI);
    assert_eq!(config.base_url, "https://api.openai.com/v1");
    assert_eq!(config.model, "gpt-3.5-turbo");
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.retry_delay, Duration::from_secs(1));
    assert_eq!(config.enable_logging, true);
    assert!(config.api_key.is_none());
}

#[tokio::test]
async fn test_custom_provider() {
    let custom_name = "my-custom-ai".to_string();
    let provider = AIProvider::Custom(custom_name.clone());

    // Test provider equality
    assert_eq!(provider, AIProvider::Custom(custom_name.clone()));
    assert_ne!(provider, AIProvider::OpenAI);

    // Test default values for custom provider
    assert_eq!(provider.default_base_url(), "");
    assert_eq!(provider.default_model(), "");
}

// Integration test that would require network access - commented out for unit testing
/*
#[tokio::test]
async fn test_actual_api_request() {
    // This test would require actual API credentials and network access
    // Only enable for integration testing
    let client = create_test_client(AIProvider::OpenAI);
    let request = create_test_request();

    match client.send_request(request).await {
        Ok(response) => {
            assert!(!response.text.is_empty());
            assert!(response.metadata.response_time_ms > 0);
        }
        Err(e) => {
            // Expected for unit tests without real API keys
            println!("Expected error in unit test: {}", e);
        }
    }
}
*/

#[tokio::test]
async fn test_request_timeout_configuration() {
    let short_timeout = Duration::from_millis(100);
    let config = AIClientConfig {
        provider: AIProvider::OpenAI,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("test-key".to_string()),
        model: "gpt-3.5-turbo".to_string(),
        timeout: short_timeout,
        max_retries: 0,
        retry_delay: Duration::from_millis(50),
        enable_logging: false,
    };

    let client = HttpAIClient::new(config).expect("Should create client with short timeout");
    assert_eq!(client.config().timeout, short_timeout);
}

#[tokio::test]
async fn test_request_retry_configuration() {
    let config = AIClientConfig {
        provider: AIProvider::OpenAI,
        base_url: "https://api.openai.com/v1".to_string(),
        api_key: Some("test-key".to_string()),
        model: "gpt-3.5-turbo".to_string(),
        timeout: Duration::from_secs(1),
        max_retries: 5,
        retry_delay: Duration::from_millis(200),
        enable_logging: false,
    };

    let client = HttpAIClient::new(config).expect("Should create client with retry config");
    assert_eq!(client.config().max_retries, 5);
    assert_eq!(client.config().retry_delay, Duration::from_millis(200));
}