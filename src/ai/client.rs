//! AI HTTP Client for external AI services
//!
//! Provides HTTP client functionality for communicating with external AI services
//! such as OpenAI, Anthropic, and other AI providers.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    time::{Duration, Instant},
};
use tracing::{debug, info, instrument};

use crate::error::{AppError, AppResult};

/// AI service provider types
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AIProvider {
    OpenAI,
    Anthropic,
    Ollama,
    Custom(String),
}

impl AIProvider {
    /// Get the default base URL for the provider
    pub fn default_base_url(&self) -> &str {
        match self {
            AIProvider::OpenAI => "https://api.openai.com/v1",
            AIProvider::Anthropic => "https://api.anthropic.com/v1",
            AIProvider::Ollama => "http://localhost:11434/api",
            AIProvider::Custom(_) => "",
        }
    }

    /// Get the default model for the provider
    pub fn default_model(&self) -> &str {
        match self {
            AIProvider::OpenAI => "gpt-3.5-turbo",
            AIProvider::Anthropic => "claude-3-haiku-20240307",
            AIProvider::Ollama => "llama2",
            AIProvider::Custom(_) => "",
        }
    }
}

/// AI client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIClientConfig {
    /// AI provider
    pub provider: AIProvider,
    /// Base URL for the API
    pub base_url: String,
    /// API key for authentication
    pub api_key: Option<String>,
    /// Default model to use
    pub model: String,
    /// Request timeout in seconds
    pub timeout: Duration,
    /// Maximum retries
    pub max_retries: u32,
    /// Retry delay in seconds
    pub retry_delay: Duration,
    /// Enable request/response logging
    pub enable_logging: bool,
}

impl Default for AIClientConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::OpenAI,
            base_url: AIProvider::OpenAI.default_base_url().to_string(),
            api_key: None,
            model: AIProvider::OpenAI.default_model().to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            retry_delay: Duration::from_secs(1),
            enable_logging: true,
        }
    }
}

/// AI request structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIRequest {
    /// The prompt or input text
    pub prompt: String,
    /// Model to use (overrides default)
    pub model: Option<String>,
    /// Maximum tokens to generate
    pub max_tokens: Option<u32>,
    /// Temperature for randomness (0.0 to 1.0)
    pub temperature: Option<f32>,
    /// Additional parameters
    pub parameters: HashMap<String, serde_json::Value>,
}

impl AIRequest {
    /// Create a new AI request with prompt
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            model: None,
            max_tokens: Some(150),
            temperature: Some(0.7),
            parameters: HashMap::new(),
        }
    }

    /// Set model for this request
    pub fn with_model(mut self, model: String) -> Self {
        self.model = Some(model);
        self
    }

    /// Set max tokens
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// Set temperature
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature.clamp(0.0, 1.0));
        self
    }

    /// Add parameter
    pub fn with_parameter(mut self, key: String, value: serde_json::Value) -> Self {
        self.parameters.insert(key, value);
        self
    }
}

/// AI response structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIResponse {
    /// Generated text response
    pub text: String,
    /// Usage information
    pub usage: Option<UsageInfo>,
    /// Response metadata
    pub metadata: ResponseMetadata,
    /// Raw response for debugging
    pub raw_response: Option<serde_json::Value>,
}

/// Usage information from AI provider
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsageInfo {
    /// Tokens used in prompt
    pub prompt_tokens: u32,
    /// Tokens generated in response
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Response metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    /// Request ID from provider
    pub request_id: Option<String>,
    /// Model used
    pub model: String,
    /// Response time in milliseconds
    pub response_time_ms: u64,
    /// Provider used
    pub provider: AIProvider,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
}

/// AI client trait for different providers
#[async_trait]
pub trait AIClient: Send + Sync {
    /// Send a request to the AI service
    async fn send_request(&self, request: AIRequest) -> AppResult<AIResponse>;

    /// Get provider information
    fn provider(&self) -> &AIProvider;

    /// Check if the client is healthy
    async fn health_check(&self) -> AppResult<bool>;

    /// Get client configuration
    fn config(&self) -> &AIClientConfig;
}

/// HTTP-based AI client implementation
pub struct HttpAIClient {
    /// HTTP client
    client: Client,
    /// Configuration
    config: AIClientConfig,
    /// Request statistics
    stats: ClientStats,
}

/// Client statistics
#[derive(Debug, Default)]
struct ClientStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    total_response_time: Duration,
    last_request_time: Option<DateTime<Utc>>,
}

impl HttpAIClient {
    /// Create a new HTTP AI client
    pub fn new(config: AIClientConfig) -> AppResult<Self> {
        let mut client_builder = Client::builder()
            .timeout(config.timeout)
            .user_agent("ai-commit-tui/0.1.0");

        // Add default headers for common providers
        let mut default_headers = reqwest::header::HeaderMap::new();
        if let Some(ref api_key) = config.api_key {
            match config.provider {
                AIProvider::OpenAI => {
                    default_headers.insert(
                        "Authorization",
                        format!("Bearer {}", api_key).parse().map_err(|e| {
                            AppError::agent(format!("Invalid API key format: {}", e))
                        })?,
                    );
                    default_headers.insert("Content-Type", "application/json".parse().unwrap());
                }
                AIProvider::Anthropic => {
                    default_headers.insert(
                        "x-api-key",
                        api_key.parse().map_err(|e| {
                            AppError::agent(format!("Invalid API key format: {}", e))
                        })?,
                    );
                    default_headers.insert("Content-Type", "application/json".parse().unwrap());
                    default_headers.insert("anthropic-version", "2023-06-01".parse().unwrap());
                }
                _ => {
                    // For other providers, use Bearer token by default
                    default_headers.insert(
                        "Authorization",
                        format!("Bearer {}", api_key).parse().map_err(|e| {
                            AppError::agent(format!("Invalid API key format: {}", e))
                        })?,
                    );
                }
            }
        }

        client_builder = client_builder.default_headers(default_headers);

        let client = client_builder.build().map_err(|e| {
            AppError::agent(format!("Failed to create HTTP client: {}", e))
        })?;

        Ok(Self {
            client,
            config,
            stats: ClientStats::default(),
        })
    }

    /// Convert generic request to provider-specific format
    fn convert_request(&self, request: &AIRequest) -> serde_json::Value {
        match self.config.provider {
            AIProvider::OpenAI => {
                serde_json::json!({
                    "model": request.model.as_ref().unwrap_or(&self.config.model),
                    "messages": [
                        {
                            "role": "user",
                            "content": request.prompt
                        }
                    ],
                    "max_tokens": request.max_tokens.unwrap_or(150),
                    "temperature": request.temperature.unwrap_or(0.7)
                })
            }
            AIProvider::Anthropic => {
                serde_json::json!({
                    "model": request.model.as_ref().unwrap_or(&self.config.model),
                    "max_tokens": request.max_tokens.unwrap_or(150),
                    "messages": [
                        {
                            "role": "user",
                            "content": request.prompt
                        }
                    ]
                })
            }
            AIProvider::Ollama => {
                serde_json::json!({
                    "model": request.model.as_ref().unwrap_or(&self.config.model),
                    "prompt": request.prompt,
                    "stream": false,
                    "options": {
                        "temperature": request.temperature.unwrap_or(0.7)
                    }
                })
            }
            AIProvider::Custom(_) => {
                // For custom providers, use a generic format
                let mut payload = serde_json::json!({
                    "model": request.model.as_ref().unwrap_or(&self.config.model),
                    "prompt": request.prompt,
                    "max_tokens": request.max_tokens.unwrap_or(150),
                    "temperature": request.temperature.unwrap_or(0.7)
                });

                // Add additional parameters
                if let serde_json::Value::Object(ref mut obj) = payload {
                    for (key, value) in &request.parameters {
                        obj.insert(key.clone(), value.clone());
                    }
                }

                payload
            }
        }
    }

    /// Extract text response from provider response
    fn extract_response(&self, response_body: &serde_json::Value) -> AppResult<String> {
        match self.config.provider {
            AIProvider::OpenAI => {
                response_body
                    .get("choices")
                    .and_then(|choices| choices.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|choice| choice.get("message"))
                    .and_then(|message| message.get("content"))
                    .and_then(|content| content.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| AppError::agent("Invalid OpenAI response format"))
            }
            AIProvider::Anthropic => {
                response_body
                    .get("content")
                    .and_then(|content| content.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|item| item.get("text"))
                    .and_then(|text| text.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| AppError::agent("Invalid Anthropic response format"))
            }
            AIProvider::Ollama => {
                response_body
                    .get("response")
                    .and_then(|response| response.as_str())
                    .map(|s| s.to_string())
                    .ok_or_else(|| AppError::agent("Invalid Ollama response format"))
            }
            AIProvider::Custom(_) => {
                // Try common response fields
                if let Some(text) = response_body
                    .get("text")
                    .or_else(|| response_body.get("response"))
                    .or_else(|| response_body.get("content"))
                    .and_then(|v| v.as_str())
                {
                    Ok(text.to_string())
                } else {
                    Err(AppError::agent("Could not extract text from custom provider response"))
                }
            }
        }
    }

    /// Extract usage information from response
    fn extract_usage(&self, response_body: &serde_json::Value) -> Option<UsageInfo> {
        match self.config.provider {
            AIProvider::OpenAI | AIProvider::Anthropic => {
                if let Some(usage) = response_body.get("usage") {
                    Some(UsageInfo {
                        prompt_tokens: usage.get("input_tokens")
                            .or_else(|| usage.get("prompt_tokens"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                        completion_tokens: usage.get("output_tokens")
                            .or_else(|| usage.get("completion_tokens"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                        total_tokens: usage.get("total_tokens")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0) as u32,
                    })
                } else {
                    None
                }
            }
            _ => None, // Other providers might not provide usage info
        }
    }

    /// Get the appropriate endpoint for the provider
    fn get_endpoint(&self) -> String {
        match self.config.provider {
            AIProvider::OpenAI => format!("{}/chat/completions", self.config.base_url),
            AIProvider::Anthropic => format!("{}/messages", self.config.base_url),
            AIProvider::Ollama => format!("{}/generate", self.config.base_url),
            AIProvider::Custom(_) => format!("{}/completions", self.config.base_url),
        }
    }

    /// Update client statistics
    fn update_stats(&mut self, success: bool, response_time: Duration) {
        self.stats.total_requests += 1;
        self.stats.total_response_time += response_time;
        self.stats.last_request_time = Some(Utc::now());

        if success {
            self.stats.successful_requests += 1;
        } else {
            self.stats.failed_requests += 1;
        }
    }
}

#[async_trait]
impl AIClient for HttpAIClient {
    #[instrument(skip(self, request))]
    async fn send_request(&self, request: AIRequest) -> AppResult<AIResponse> {
        let start_time = Instant::now();

        info!("Sending AI request to {:?} provider", self.config.provider);
        if self.config.enable_logging {
            debug!("Request prompt: {}", request.prompt);
        }

        let endpoint = self.get_endpoint();
        let payload = self.convert_request(&request);

        // Implement retry logic
        let mut last_error = None;
        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                tokio::time::sleep(self.config.retry_delay).await;
                debug!("Retry attempt {} for AI request", attempt);
            }

            match self.client.post(&endpoint).json(&payload).send().await {
                Ok(response) => {
                    let status = response.status();
                    if status.is_success() {
                        match response.json::<serde_json::Value>().await {
                            Ok(response_body) => {
                                let response_time = start_time.elapsed();

                                let text = self.extract_response(&response_body)?;
                                let usage = self.extract_usage(&response_body);

                                let ai_response = AIResponse {
                                    text,
                                    usage,
                                    metadata: ResponseMetadata {
                                        request_id: None, // Could extract from headers
                                        model: request.model.unwrap_or_else(|| self.config.model.clone()),
                                        response_time_ms: response_time.as_millis() as u64,
                                        provider: self.config.provider.clone(),
                                        timestamp: Utc::now(),
                                    },
                                    raw_response: if self.config.enable_logging {
                                        Some(response_body)
                                    } else {
                                        None
                                    },
                                };

                                info!(
                                    "AI request completed in {}ms",
                                    response_time.as_millis()
                                );

                                return Ok(ai_response);
                            }
                            Err(e) => {
                                last_error = Some(AppError::agent(format!(
                                    "Failed to parse AI response: {}",
                                    e
                                )));
                            }
                        }
                    } else {
                        let error_body = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        last_error = Some(AppError::agent(format!(
                            "AI request failed with status {}: {}",
                            status, error_body
                        )));
                    }
                }
                Err(e) => {
                    last_error = Some(AppError::agent(format!(
                        "HTTP request failed: {}",
                        e
                    )));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| AppError::agent("AI request failed after all retries")))
    }

    fn provider(&self) -> &AIProvider {
        &self.config.provider
    }

    async fn health_check(&self) -> AppResult<bool> {
        // Simple health check - try to get a small response
        let health_request = AIRequest::new("test".to_string())
            .with_max_tokens(1)
            .with_temperature(0.0);

        match self.send_request(health_request).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    fn config(&self) -> &AIClientConfig {
        &self.config
    }
}

impl HttpAIClient {
    /// Get client statistics
    pub fn stats(&self) -> &ClientStats {
        &self.stats
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.stats.total_requests == 0 {
            0.0
        } else {
            self.stats.successful_requests as f64 / self.stats.total_requests as f64
        }
    }

    /// Get average response time
    pub fn average_response_time(&self) -> Duration {
        if self.stats.total_requests == 0 {
            Duration::ZERO
        } else {
            self.stats.total_response_time / self.stats.total_requests as u32
        }
    }
}