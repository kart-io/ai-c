//! Timeout and Retry Strategy for Agent Tasks
//!
//! Provides configurable timeout and retry mechanisms for agent task execution
//! with exponential backoff, circuit breaker patterns, and failure tracking.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{sync::RwLock, time::timeout};
use tracing::{debug, error, info, warn};

use crate::error::{AppError, AppResult};

/// Retry strategy configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay between retries
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Multiplier for exponential backoff
    pub backoff_multiplier: f64,
    /// Whether to use jitter in delays
    pub use_jitter: bool,
    /// Timeout for each individual attempt
    pub per_attempt_timeout: Duration,
    /// Total timeout for all attempts combined
    pub total_timeout: Duration,
    /// Types of errors that should trigger retries
    pub retryable_errors: Vec<RetryableError>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            backoff_multiplier: 2.0,
            use_jitter: true,
            per_attempt_timeout: Duration::from_secs(30),
            total_timeout: Duration::from_secs(300), // 5 minutes
            retryable_errors: vec![
                RetryableError::NetworkError,
                RetryableError::ServiceUnavailable,
                RetryableError::RateLimited,
                RetryableError::InternalServerError,
            ],
        }
    }
}

/// Types of errors that can be retried
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RetryableError {
    /// Network connectivity issues
    NetworkError,
    /// Service temporarily unavailable
    ServiceUnavailable,
    /// Rate limiting by external service
    RateLimited,
    /// Internal server errors (5xx)
    InternalServerError,
    /// Timeout errors
    Timeout,
    /// Resource temporarily unavailable
    ResourceBusy,
    /// Agent temporarily unavailable
    AgentUnavailable,
    /// Custom retryable error
    Custom(String),
}

/// Retry attempt information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryAttempt {
    /// Attempt number (1-based)
    pub attempt_number: u32,
    /// Start time of this attempt
    pub start_time: Instant,
    /// Duration of this attempt
    pub duration: Option<Duration>,
    /// Error that occurred (if any)
    pub error: Option<String>,
    /// Whether this attempt succeeded
    pub succeeded: bool,
}

/// Retry execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryResult<T> {
    /// Final result (if successful)
    pub result: Option<T>,
    /// Final error (if failed)
    pub error: Option<AppError>,
    /// Total number of attempts made
    pub total_attempts: u32,
    /// Total execution time
    pub total_duration: Duration,
    /// Details of each attempt
    pub attempts: Vec<RetryAttempt>,
    /// Whether the operation ultimately succeeded
    pub succeeded: bool,
}

/// Trait for operations that can be retried
#[async_trait]
pub trait RetryableOperation<T>: Send + Sync {
    /// Execute the operation
    async fn execute(&mut self) -> AppResult<T>;

    /// Check if an error is retryable
    fn is_retryable(&self, error: &AppError) -> bool;

    /// Operation name for logging
    fn operation_name(&self) -> &str;
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Circuit is closed, allowing requests
    Closed,
    /// Circuit is open, rejecting requests
    Open,
    /// Circuit is half-open, testing if service recovered
    HalfOpen,
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Time to wait before trying to close circuit
    pub recovery_timeout: Duration,
    /// Number of successful requests needed to close circuit
    pub success_threshold: u32,
    /// Time window for failure counting
    pub failure_window: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(60),
            success_threshold: 3,
            failure_window: Duration::from_secs(60),
        }
    }
}

/// Circuit breaker for preventing cascading failures
#[derive(Debug)]
pub struct CircuitBreaker {
    /// Circuit breaker configuration
    config: CircuitBreakerConfig,
    /// Current state
    state: Arc<RwLock<CircuitBreakerState>>,
    /// Failure count in current window
    failure_count: Arc<RwLock<u32>>,
    /// Success count in half-open state
    success_count: Arc<RwLock<u32>>,
    /// Last failure time
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    /// Statistics
    stats: Arc<RwLock<CircuitBreakerStats>>,
}

/// Circuit breaker statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CircuitBreakerStats {
    /// Total requests processed
    pub total_requests: u64,
    /// Total failures
    pub total_failures: u64,
    /// Total successful requests
    pub total_successes: u64,
    /// Number of times circuit opened
    pub circuit_opened_count: u64,
    /// Number of times circuit closed
    pub circuit_closed_count: u64,
    /// Current state duration
    pub current_state_start: Option<Instant>,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(CircuitBreakerState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            success_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            stats: Arc::new(RwLock::new(CircuitBreakerStats {
                current_state_start: Some(Instant::now()),
                ..Default::default()
            })),
        }
    }

    /// Check if requests should be allowed
    pub async fn can_proceed(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::Open => {
                // Check if recovery timeout has passed
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() >= self.config.recovery_timeout {
                        drop(state);
                        self.transition_to_half_open().await;
                        return true;
                    }
                }
                false
            }
            CircuitBreakerState::HalfOpen => true,
        }
    }

    /// Record a successful request
    pub async fn record_success(&self) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.total_successes += 1;

        let state = self.state.read().await;
        match *state {
            CircuitBreakerState::HalfOpen => {
                drop(state);
                let mut success_count = self.success_count.write().await;
                *success_count += 1;

                if *success_count >= self.config.success_threshold {
                    self.transition_to_closed().await;
                }
            }
            CircuitBreakerState::Closed => {
                // Reset failure count on success
                *self.failure_count.write().await = 0;
            }
            _ => {}
        }
    }

    /// Record a failed request
    pub async fn record_failure(&self) {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        stats.total_failures += 1;

        *self.last_failure_time.write().await = Some(Instant::now());

        let state = self.state.read().await;
        match *state {
            CircuitBreakerState::Closed => {
                drop(state);
                let mut failure_count = self.failure_count.write().await;
                *failure_count += 1;

                if *failure_count >= self.config.failure_threshold {
                    self.transition_to_open().await;
                }
            }
            CircuitBreakerState::HalfOpen => {
                drop(state);
                self.transition_to_open().await;
            }
            _ => {}
        }
    }

    /// Transition to open state
    async fn transition_to_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Open;

        let mut stats = self.stats.write().await;
        stats.circuit_opened_count += 1;
        stats.current_state_start = Some(Instant::now());

        warn!("Circuit breaker opened due to repeated failures");
    }

    /// Transition to half-open state
    async fn transition_to_half_open(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::HalfOpen;

        *self.success_count.write().await = 0;

        let mut stats = self.stats.write().await;
        stats.current_state_start = Some(Instant::now());

        info!("Circuit breaker transitioned to half-open state");
    }

    /// Transition to closed state
    async fn transition_to_closed(&self) {
        let mut state = self.state.write().await;
        *state = CircuitBreakerState::Closed;

        *self.failure_count.write().await = 0;
        *self.success_count.write().await = 0;

        let mut stats = self.stats.write().await;
        stats.circuit_closed_count += 1;
        stats.current_state_start = Some(Instant::now());

        info!("Circuit breaker closed - service recovered");
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitBreakerState {
        *self.state.read().await
    }

    /// Get statistics
    pub async fn get_stats(&self) -> CircuitBreakerStats {
        self.stats.read().await.clone()
    }
}

/// Timeout and retry executor
pub struct TimeoutRetryExecutor {
    /// Default retry configuration
    default_config: RetryConfig,
    /// Circuit breakers per operation type
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Executor statistics
    stats: Arc<RwLock<ExecutorStats>>,
}

/// Executor statistics
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ExecutorStats {
    /// Total operations executed
    pub total_operations: u64,
    /// Total successful operations
    pub successful_operations: u64,
    /// Total failed operations
    pub failed_operations: u64,
    /// Total retry attempts made
    pub total_retry_attempts: u64,
    /// Average execution time
    pub average_execution_time: Duration,
    /// Operations by type
    pub operations_by_type: HashMap<String, u64>,
}

impl TimeoutRetryExecutor {
    /// Create a new timeout retry executor
    pub fn new(default_config: RetryConfig) -> Self {
        Self {
            default_config,
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(ExecutorStats::default())),
        }
    }

    /// Execute an operation with retry and timeout
    pub async fn execute<T>(
        &self,
        mut operation: Box<dyn RetryableOperation<T>>,
        config: Option<RetryConfig>,
    ) -> RetryResult<T> {
        let config = config.unwrap_or_else(|| self.default_config.clone());
        let start_time = Instant::now();
        let operation_name = operation.operation_name().to_string();

        debug!("Starting operation '{}' with retry config", operation_name);

        // Check circuit breaker
        let circuit_breaker = self.get_or_create_circuit_breaker(&operation_name).await;
        if !circuit_breaker.can_proceed().await {
            warn!("Circuit breaker is open for operation '{}'", operation_name);
            return RetryResult {
                result: None,
                error: Some(AppError::agent("Circuit breaker is open")),
                total_attempts: 0,
                total_duration: Duration::ZERO,
                attempts: vec![],
                succeeded: false,
            };
        }

        let mut attempts = Vec::new();
        let mut current_delay = config.initial_delay;

        // Execute with total timeout
        let total_result = timeout(config.total_timeout, async {
            for attempt_num in 1..=config.max_attempts {
                let attempt_start = Instant::now();

                debug!("Attempt {} for operation '{}'", attempt_num, operation_name);

                // Execute single attempt with per-attempt timeout
                let attempt_result = timeout(config.per_attempt_timeout, operation.execute()).await;

                let attempt_duration = attempt_start.elapsed();
                let mut attempt = RetryAttempt {
                    attempt_number: attempt_num,
                    start_time: attempt_start,
                    duration: Some(attempt_duration),
                    error: None,
                    succeeded: false,
                };

                match attempt_result {
                    Ok(Ok(result)) => {
                        // Success
                        attempt.succeeded = true;
                        attempts.push(attempt);

                        circuit_breaker.record_success().await;
                        self.update_stats(&operation_name, true, start_time.elapsed()).await;

                        info!(
                            "Operation '{}' succeeded on attempt {} in {:?}",
                            operation_name, attempt_num, attempt_duration
                        );

                        return Ok(result);
                    }
                    Ok(Err(error)) => {
                        // Operation failed
                        attempt.error = Some(error.to_string());
                        attempts.push(attempt);

                        if !operation.is_retryable(&error) || attempt_num == config.max_attempts {
                            // Non-retryable error or last attempt
                            circuit_breaker.record_failure().await;
                            return Err(error);
                        }

                        warn!(
                            "Attempt {} failed for operation '{}': {} (retrying in {:?})",
                            attempt_num, operation_name, error, current_delay
                        );
                    }
                    Err(_) => {
                        // Timeout
                        let timeout_error = AppError::agent("Operation timed out");
                        attempt.error = Some(timeout_error.to_string());
                        attempts.push(attempt);

                        if attempt_num == config.max_attempts {
                            circuit_breaker.record_failure().await;
                            return Err(timeout_error);
                        }

                        warn!(
                            "Attempt {} timed out for operation '{}' (retrying in {:?})",
                            attempt_num, operation_name, current_delay
                        );
                    }
                }

                // Wait before next attempt (except for last attempt)
                if attempt_num < config.max_attempts {
                    let delay = if config.use_jitter {
                        self.add_jitter(current_delay)
                    } else {
                        current_delay
                    };

                    tokio::time::sleep(delay).await;

                    // Update delay for next attempt (exponential backoff)
                    current_delay = Duration::from_millis(
                        ((current_delay.as_millis() as f64) * config.backoff_multiplier) as u64
                    ).min(config.max_delay);
                }
            }

            Err(AppError::agent("All retry attempts failed"))
        }).await;

        let total_duration = start_time.elapsed();

        match total_result {
            Ok(Ok(result)) => {
                RetryResult {
                    result: Some(result),
                    error: None,
                    total_attempts: attempts.len() as u32,
                    total_duration,
                    attempts,
                    succeeded: true,
                }
            }
            Ok(Err(_)) | Err(_) => {
                let final_error = total_result.err()
                    .map(|_| AppError::agent("Total timeout exceeded"))
                    .or_else(|| total_result.ok().and_then(|r| r.err()))
                    .unwrap_or_else(|| AppError::agent("Unknown error"));

                self.update_stats(&operation_name, false, total_duration).await;

                error!(
                    "Operation '{}' failed after {} attempts in {:?}: {}",
                    operation_name, attempts.len(), total_duration, final_error
                );

                RetryResult {
                    result: None,
                    error: Some(final_error),
                    total_attempts: attempts.len() as u32,
                    total_duration,
                    attempts,
                    succeeded: false,
                }
            }
        }
    }

    /// Add jitter to delay to prevent thundering herd
    fn add_jitter(&self, delay: Duration) -> Duration {
        use rand::Rng;
        let jitter_factor = rand::thread_rng().gen_range(0.5..1.5);
        Duration::from_millis((delay.as_millis() as f64 * jitter_factor) as u64)
    }

    /// Get or create circuit breaker for operation
    async fn get_or_create_circuit_breaker(&self, operation_name: &str) -> Arc<CircuitBreaker> {
        let mut circuit_breakers = self.circuit_breakers.write().await;

        if let Some(cb) = circuit_breakers.get(operation_name) {
            return Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()));
        }

        let circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig::default());
        circuit_breakers.insert(operation_name.to_string(), circuit_breaker);

        Arc::new(CircuitBreaker::new(CircuitBreakerConfig::default()))
    }

    /// Update executor statistics
    async fn update_stats(&self, operation_name: &str, success: bool, duration: Duration) {
        let mut stats = self.stats.write().await;

        stats.total_operations += 1;
        if success {
            stats.successful_operations += 1;
        } else {
            stats.failed_operations += 1;
        }

        *stats.operations_by_type.entry(operation_name.to_string()).or_insert(0) += 1;

        // Update average execution time
        let total_time = stats.average_execution_time.as_nanos() as u64 * (stats.total_operations - 1) + duration.as_nanos() as u64;
        stats.average_execution_time = Duration::from_nanos(total_time / stats.total_operations);
    }

    /// Get executor statistics
    pub async fn get_stats(&self) -> ExecutorStats {
        self.stats.read().await.clone()
    }

    /// Get circuit breaker states
    pub async fn get_circuit_breaker_states(&self) -> HashMap<String, CircuitBreakerState> {
        let circuit_breakers = self.circuit_breakers.read().await;
        let mut states = HashMap::new();

        for (name, cb) in circuit_breakers.iter() {
            states.insert(name.clone(), cb.get_state().await);
        }

        states
    }
}

impl Default for TimeoutRetryExecutor {
    fn default() -> Self {
        Self::new(RetryConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockOperation {
        name: String,
        attempts_to_succeed: u32,
        current_attempt: u32,
    }

    impl MockOperation {
        fn new(name: &str, attempts_to_succeed: u32) -> Self {
            Self {
                name: name.to_string(),
                attempts_to_succeed,
                current_attempt: 0,
            }
        }
    }

    #[async_trait]
    impl RetryableOperation<String> for MockOperation {
        async fn execute(&mut self) -> AppResult<String> {
            self.current_attempt += 1;

            if self.current_attempt < self.attempts_to_succeed {
                Err(AppError::agent("Mock failure"))
            } else {
                Ok(format!("Success after {} attempts", self.current_attempt))
            }
        }

        fn is_retryable(&self, _error: &AppError) -> bool {
            true // Always retryable for testing
        }

        fn operation_name(&self) -> &str {
            &self.name
        }
    }

    #[tokio::test]
    async fn test_retry_success_on_second_attempt() {
        let executor = TimeoutRetryExecutor::default();
        let operation = Box::new(MockOperation::new("test_op", 2));

        let config = RetryConfig {
            max_attempts: 3,
            initial_delay: Duration::from_millis(10),
            ..RetryConfig::default()
        };

        let result = executor.execute(operation, Some(config)).await;

        assert!(result.succeeded);
        assert_eq!(result.total_attempts, 2);
        assert!(result.result.is_some());
        assert_eq!(result.result.unwrap(), "Success after 2 attempts");
    }

    #[tokio::test]
    async fn test_retry_exhaustion() {
        let executor = TimeoutRetryExecutor::default();
        let operation = Box::new(MockOperation::new("test_op", 5)); // Needs 5 attempts

        let config = RetryConfig {
            max_attempts: 3, // Only allow 3 attempts
            initial_delay: Duration::from_millis(10),
            ..RetryConfig::default()
        };

        let result = executor.execute(operation, Some(config)).await;

        assert!(!result.succeeded);
        assert_eq!(result.total_attempts, 3);
        assert!(result.result.is_none());
        assert!(result.error.is_some());
    }

    #[tokio::test]
    async fn test_circuit_breaker() {
        let circuit_breaker = CircuitBreaker::new(CircuitBreakerConfig {
            failure_threshold: 2,
            ..CircuitBreakerConfig::default()
        });

        // Initially closed
        assert_eq!(circuit_breaker.get_state().await, CircuitBreakerState::Closed);
        assert!(circuit_breaker.can_proceed().await);

        // First failure
        circuit_breaker.record_failure().await;
        assert_eq!(circuit_breaker.get_state().await, CircuitBreakerState::Closed);

        // Second failure should open circuit
        circuit_breaker.record_failure().await;
        assert_eq!(circuit_breaker.get_state().await, CircuitBreakerState::Open);
        assert!(!circuit_breaker.can_proceed().await);
    }
}