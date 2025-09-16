//! Failure recovery system for agents
//!
//! Provides fault detection, circuit breaker patterns, and configurable
//! recovery strategies to maintain system resilience and availability.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use tracing::{error, info, warn};

use super::HealthStatus;

/// Failure recovery configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureRecoveryConfig {
    /// Enable automatic recovery
    pub enable_auto_recovery: bool,
    /// Maximum recovery attempts per agent
    pub max_recovery_attempts: u32,
    /// Recovery attempt interval
    pub recovery_interval: Duration,
    /// Circuit breaker configuration
    pub circuit_breaker: CircuitBreakerConfig,
    /// Failure detection configuration
    pub failure_detection: FailureDetectionConfig,
}

impl Default for FailureRecoveryConfig {
    fn default() -> Self {
        Self {
            enable_auto_recovery: true,
            max_recovery_attempts: 3,
            recovery_interval: Duration::from_secs(60),
            circuit_breaker: CircuitBreakerConfig::default(),
            failure_detection: FailureDetectionConfig::default(),
        }
    }
}

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Failure threshold to open circuit
    pub failure_threshold: u32,
    /// Success threshold to close circuit
    pub success_threshold: u32,
    /// Timeout before half-open state
    pub timeout: Duration,
    /// Reset timeout for closed state
    pub reset_timeout: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(60),
            reset_timeout: Duration::from_secs(300),
        }
    }
}

/// Failure detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureDetectionConfig {
    /// Detection interval
    pub detection_interval: Duration,
    /// Health check timeout threshold
    pub health_check_timeout: Duration,
    /// Task timeout threshold
    pub task_timeout_threshold: Duration,
    /// Communication error threshold
    pub communication_error_threshold: u32,
}

impl Default for FailureDetectionConfig {
    fn default() -> Self {
        Self {
            detection_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(10),
            task_timeout_threshold: Duration::from_secs(120),
            communication_error_threshold: 3,
        }
    }
}

/// Types of failures that can occur
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FailureType {
    /// Task execution timeout
    TaskTimeout,
    /// Health check failed
    HealthCheckFailed,
    /// Communication error
    CommunicationError,
    /// Resource exhaustion
    ResourceExhaustion,
    /// Agent initialization failed
    InitializationFailed,
    /// Agent crashed or stopped responding
    AgentCrash,
    /// Memory or CPU limits exceeded
    ResourceLimitExceeded,
    /// Custom failure type
    Custom(String),
}

/// Failure information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureInfo {
    /// Agent identifier
    pub agent_id: String,
    /// Type of failure
    pub failure_type: FailureType,
    /// Error message
    pub error_message: String,
    /// When the failure occurred
    pub occurred_at: SystemTime,
    /// Additional context
    pub context: HashMap<String, serde_json::Value>,
    /// Severity level
    pub severity: FailureSeverity,
}

impl FailureInfo {
    pub fn new(agent_id: String, failure_type: FailureType, error_message: String) -> Self {
        Self {
            agent_id,
            failure_type,
            error_message,
            occurred_at: SystemTime::now(),
            context: HashMap::new(),
            severity: FailureSeverity::Medium,
        }
    }

    pub fn with_context(mut self, key: String, value: serde_json::Value) -> Self {
        self.context.insert(key, value);
        self
    }

    pub fn with_severity(mut self, severity: FailureSeverity) -> Self {
        self.severity = severity;
        self
    }
}

/// Failure severity levels
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum FailureSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Recovery error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum RecoveryError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Recovery strategy not found for failure type: {0:?}")]
    StrategyNotFound(FailureType),

    #[error("Recovery failed: {0}")]
    RecoveryFailed(String),

    #[error("Circuit breaker is open for agent: {0}")]
    CircuitBreakerOpen(String),

    #[error("Maximum recovery attempts exceeded: {0}")]
    MaxAttemptsExceeded(String),

    #[error("Recovery timeout: {0}")]
    Timeout(String),

    #[error("Internal recovery error: {0}")]
    InternalError(String),
}

/// Circuit breaker states
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CircuitBreakerState {
    /// Circuit is closed - normal operation
    Closed,
    /// Circuit is open - requests are blocked
    Open,
    /// Circuit is half-open - testing if service is recovered
    HalfOpen,
}

/// Circuit breaker implementation
#[derive(Debug, Clone)]
pub struct CircuitBreaker {
    /// Agent identifier
    pub agent_id: String,
    /// Current state
    pub state: CircuitBreakerState,
    /// Consecutive failure count
    pub failure_count: u32,
    /// Consecutive success count (for half-open state)
    pub success_count: u32,
    /// Last failure timestamp
    pub last_failure_time: Option<SystemTime>,
    /// Last state change timestamp
    pub last_state_change: SystemTime,
    /// Configuration
    pub config: CircuitBreakerConfig,
}

impl CircuitBreaker {
    pub fn new(agent_id: String, config: CircuitBreakerConfig) -> Self {
        Self {
            agent_id,
            state: CircuitBreakerState::Closed,
            failure_count: 0,
            success_count: 0,
            last_failure_time: None,
            last_state_change: SystemTime::now(),
            config,
        }
    }

    /// Record a successful operation
    pub fn record_success(&mut self) {
        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count = 0;
            }
            CircuitBreakerState::HalfOpen => {
                self.success_count += 1;
                if self.success_count >= self.config.success_threshold {
                    self.state = CircuitBreakerState::Closed;
                    self.failure_count = 0;
                    self.success_count = 0;
                    self.last_state_change = SystemTime::now();
                    info!("Circuit breaker closed for agent: {}", self.agent_id);
                }
            }
            CircuitBreakerState::Open => {
                // Ignore successes when open
            }
        }
    }

    /// Record a failed operation
    pub fn record_failure(&mut self) {
        self.last_failure_time = Some(SystemTime::now());

        match self.state {
            CircuitBreakerState::Closed => {
                self.failure_count += 1;
                if self.failure_count >= self.config.failure_threshold {
                    self.state = CircuitBreakerState::Open;
                    self.last_state_change = SystemTime::now();
                    warn!("Circuit breaker opened for agent: {}", self.agent_id);
                }
            }
            CircuitBreakerState::HalfOpen => {
                self.state = CircuitBreakerState::Open;
                self.failure_count += 1;
                self.success_count = 0;
                self.last_state_change = SystemTime::now();
                warn!("Circuit breaker reopened for agent: {}", self.agent_id);
            }
            CircuitBreakerState::Open => {
                self.failure_count += 1;
            }
        }
    }

    /// Check if operation is allowed
    pub fn is_call_allowed(&mut self) -> bool {
        match self.state {
            CircuitBreakerState::Closed => true,
            CircuitBreakerState::HalfOpen => true,
            CircuitBreakerState::Open => {
                // Check if timeout has elapsed
                if let Ok(elapsed) = self.last_state_change.elapsed() {
                    if elapsed >= self.config.timeout {
                        self.state = CircuitBreakerState::HalfOpen;
                        self.success_count = 0;
                        self.last_state_change = SystemTime::now();
                        info!("Circuit breaker half-opened for agent: {}", self.agent_id);
                        return true;
                    }
                }
                false
            }
        }
    }

    /// Get current state
    pub fn get_state(&self) -> CircuitBreakerState {
        self.state.clone()
    }
}

/// Recovery strategy trait
#[async_trait]
pub trait RecoveryStrategy: Send + Sync {
    /// Attempt to recover from a failure
    async fn recover(&self, agent_id: &str, failure: &FailureInfo) -> Result<(), RecoveryError>;

    /// Get strategy name
    fn name(&self) -> &str;

    /// Check if this strategy can handle the failure type
    fn can_handle(&self, failure_type: &FailureType) -> bool;

    /// Get strategy priority (higher numbers executed first)
    fn priority(&self) -> u8 {
        100
    }
}

/// Restart recovery strategy - attempts to restart the agent
#[derive(Debug, Default)]
pub struct RestartRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for RestartRecoveryStrategy {
    async fn recover(&self, agent_id: &str, failure: &FailureInfo) -> Result<(), RecoveryError> {
        info!("Attempting restart recovery for agent: {}", agent_id);

        // Simulate restart process
        tokio::time::sleep(Duration::from_millis(500)).await;

        match failure.failure_type {
            FailureType::AgentCrash | FailureType::InitializationFailed => {
                info!("Agent {} restart completed", agent_id);
                Ok(())
            }
            _ => Err(RecoveryError::RecoveryFailed(format!(
                "Restart strategy cannot handle failure type: {:?}",
                failure.failure_type
            ))),
        }
    }

    fn name(&self) -> &str {
        "Restart"
    }

    fn can_handle(&self, failure_type: &FailureType) -> bool {
        matches!(
            failure_type,
            FailureType::AgentCrash | FailureType::InitializationFailed | FailureType::ResourceLimitExceeded
        )
    }

    fn priority(&self) -> u8 {
        80
    }
}

/// Degraded mode recovery strategy - switches agent to limited functionality
#[derive(Debug, Default)]
pub struct DegradedModeRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for DegradedModeRecoveryStrategy {
    async fn recover(&self, agent_id: &str, failure: &FailureInfo) -> Result<(), RecoveryError> {
        info!("Switching agent {} to degraded mode", agent_id);

        // Simulate switching to degraded mode
        tokio::time::sleep(Duration::from_millis(200)).await;

        match failure.failure_type {
            FailureType::ResourceExhaustion | FailureType::ResourceLimitExceeded => {
                info!("Agent {} switched to degraded mode", agent_id);
                Ok(())
            }
            _ => Ok(()), // Can handle most failure types by degrading
        }
    }

    fn name(&self) -> &str {
        "DegradedMode"
    }

    fn can_handle(&self, _failure_type: &FailureType) -> bool {
        true // Can handle most failure types
    }

    fn priority(&self) -> u8 {
        60
    }
}

/// Failover recovery strategy - switches to backup agent
#[derive(Debug, Default)]
pub struct FailoverRecoveryStrategy;

#[async_trait]
impl RecoveryStrategy for FailoverRecoveryStrategy {
    async fn recover(&self, agent_id: &str, failure: &FailureInfo) -> Result<(), RecoveryError> {
        info!("Attempting failover for agent: {}", agent_id);

        // Simulate failover process
        tokio::time::sleep(Duration::from_millis(300)).await;

        match failure.severity {
            FailureSeverity::High | FailureSeverity::Critical => {
                info!("Failover completed for agent {}", agent_id);
                Ok(())
            }
            _ => Err(RecoveryError::RecoveryFailed(
                "Failover only for high severity failures".to_string()
            )),
        }
    }

    fn name(&self) -> &str {
        "Failover"
    }

    fn can_handle(&self, failure_type: &FailureType) -> bool {
        matches!(
            failure_type,
            FailureType::AgentCrash | FailureType::CommunicationError | FailureType::TaskTimeout
        )
    }

    fn priority(&self) -> u8 {
        90
    }
}

/// Failure detector for proactive failure detection
#[derive(Debug)]
pub struct FailureDetector {
    config: FailureDetectionConfig,
    detected_failures: Arc<RwLock<HashMap<String, Vec<FailureInfo>>>>,
}

impl FailureDetector {
    pub fn new(config: FailureDetectionConfig) -> Self {
        Self {
            config,
            detected_failures: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Detect potential failures in agent
    pub async fn detect_failures(&self, agent_id: &str, health_status: &HealthStatus) -> Vec<FailureInfo> {
        let mut failures = Vec::new();

        match health_status {
            HealthStatus::Unhealthy(msg) => {
                let failure = FailureInfo::new(
                    agent_id.to_string(),
                    FailureType::HealthCheckFailed,
                    msg.clone(),
                ).with_severity(FailureSeverity::High);
                failures.push(failure);
            }
            HealthStatus::Degraded(msg) => {
                let failure = FailureInfo::new(
                    agent_id.to_string(),
                    FailureType::ResourceExhaustion,
                    msg.clone(),
                ).with_severity(FailureSeverity::Medium);
                failures.push(failure);
            }
            _ => {}
        }

        // Store detected failures
        if !failures.is_empty() {
            let mut detected = self.detected_failures.write().await;
            detected.entry(agent_id.to_string()).or_insert_with(Vec::new).extend(failures.clone());
        }

        failures
    }

    /// Get detected failures for an agent
    pub async fn get_failures(&self, agent_id: &str) -> Vec<FailureInfo> {
        let detected = self.detected_failures.read().await;
        detected.get(agent_id).cloned().unwrap_or_default()
    }

    /// Clear failures for an agent
    pub async fn clear_failures(&self, agent_id: &str) {
        let mut detected = self.detected_failures.write().await;
        detected.remove(agent_id);
    }
}

/// Failure recovery manager
pub struct FailureRecoveryManager {
    /// Recovery strategies by failure type
    recovery_strategies: Arc<RwLock<HashMap<FailureType, Vec<Arc<dyn RecoveryStrategy>>>>>,
    /// Circuit breakers by agent ID
    circuit_breakers: Arc<RwLock<HashMap<String, CircuitBreaker>>>,
    /// Failure detector
    failure_detector: Arc<FailureDetector>,
    /// Configuration
    config: FailureRecoveryConfig,
    /// Recovery statistics
    stats: Arc<RwLock<RecoveryStats>>,
}

/// Recovery statistics
#[derive(Debug, Default)]
struct RecoveryStats {
    total_failures: u64,
    successful_recoveries: u64,
    failed_recoveries: u64,
    circuit_breaker_activations: u64,
}

impl FailureRecoveryManager {
    /// Create a new failure recovery manager
    pub fn new(config: FailureRecoveryConfig) -> Self {
        let failure_detector = Arc::new(FailureDetector::new(config.failure_detection.clone()));

        let manager = Self {
            recovery_strategies: Arc::new(RwLock::new(HashMap::new())),
            circuit_breakers: Arc::new(RwLock::new(HashMap::new())),
            failure_detector,
            config,
            stats: Arc::new(RwLock::new(RecoveryStats::default())),
        };

        // Register default recovery strategies
        tokio::spawn(async move {
            // Note: In a real implementation, these would be registered during initialization
        });

        manager
    }

    /// Create with default configuration
    pub fn new_with_defaults() -> Self {
        Self::new(FailureRecoveryConfig::default())
    }

    /// Register a recovery strategy
    pub async fn register_strategy(&self, failure_type: FailureType, strategy: Arc<dyn RecoveryStrategy>) {
        info!("Registering recovery strategy '{}' for failure type: {:?}", strategy.name(), failure_type);

        let mut strategies = self.recovery_strategies.write().await;
        let type_strategies = strategies.entry(failure_type).or_insert_with(Vec::new);
        type_strategies.push(strategy);

        // Sort by priority (higher priority first)
        type_strategies.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Handle a failure
    pub async fn handle_failure(&self, agent_id: &str, failure: FailureInfo) -> Result<(), RecoveryError> {
        info!("Handling failure for agent {}: {:?}", agent_id, failure.failure_type);

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_failures += 1;
        }

        // Check circuit breaker
        let circuit_breaker_open = {
            let mut breakers = self.circuit_breakers.write().await;
            let breaker = breakers
                .entry(agent_id.to_string())
                .or_insert_with(|| CircuitBreaker::new(agent_id.to_string(), self.config.circuit_breaker.clone()));

            breaker.record_failure();

            if !breaker.is_call_allowed() {
                let mut stats = self.stats.write().await;
                stats.circuit_breaker_activations += 1;
                true
            } else {
                false
            }
        };

        if circuit_breaker_open {
            warn!("Circuit breaker is open for agent: {}", agent_id);
            return Err(RecoveryError::CircuitBreakerOpen(agent_id.to_string()));
        }

        // Skip recovery if not enabled
        if !self.config.enable_auto_recovery {
            return Ok(());
        }

        // Find and execute recovery strategies
        let strategies = {
            let strats = self.recovery_strategies.read().await;
            strats.get(&failure.failure_type).cloned().unwrap_or_default()
        };

        if strategies.is_empty() {
            warn!("No recovery strategy found for failure type: {:?}", failure.failure_type);
            return Err(RecoveryError::StrategyNotFound(failure.failure_type));
        }

        // Try recovery strategies in priority order
        for strategy in strategies {
            if strategy.can_handle(&failure.failure_type) {
                match strategy.recover(agent_id, &failure).await {
                    Ok(()) => {
                        info!("Recovery successful using strategy: {}", strategy.name());

                        // Update circuit breaker
                        {
                            let mut breakers = self.circuit_breakers.write().await;
                            if let Some(breaker) = breakers.get_mut(agent_id) {
                                breaker.record_success();
                            }
                        }

                        // Update statistics
                        {
                            let mut stats = self.stats.write().await;
                            stats.successful_recoveries += 1;
                        }

                        return Ok(());
                    }
                    Err(e) => {
                        warn!("Recovery strategy '{}' failed: {}", strategy.name(), e);
                        continue;
                    }
                }
            }
        }

        // All strategies failed
        {
            let mut stats = self.stats.write().await;
            stats.failed_recoveries += 1;
        }

        Err(RecoveryError::RecoveryFailed(format!(
            "All recovery strategies failed for agent: {}",
            agent_id
        )))
    }

    /// Get circuit breaker state
    pub async fn get_circuit_breaker_state(&self, agent_id: &str) -> CircuitBreakerState {
        let breakers = self.circuit_breakers.read().await;
        breakers
            .get(agent_id)
            .map(|b| b.get_state())
            .unwrap_or(CircuitBreakerState::Closed)
    }

    /// Register an agent with the recovery system
    pub async fn register_agent(&self, agent_id: String) -> Result<(), RecoveryError> {
        info!("Registering agent with recovery system: {}", agent_id);

        let mut breakers = self.circuit_breakers.write().await;
        breakers.insert(
            agent_id.clone(),
            CircuitBreaker::new(agent_id, self.config.circuit_breaker.clone()),
        );

        Ok(())
    }

    /// Unregister an agent
    pub async fn unregister_agent(&self, agent_id: &str) -> Result<(), RecoveryError> {
        info!("Unregistering agent from recovery system: {}", agent_id);

        {
            let mut breakers = self.circuit_breakers.write().await;
            breakers.remove(agent_id);
        }

        // Clear failure history
        self.failure_detector.clear_failures(agent_id).await;

        Ok(())
    }

    /// Perform proactive failure detection
    pub async fn detect_failures(&self, agent_id: &str, health_status: &HealthStatus) -> Vec<FailureInfo> {
        self.failure_detector.detect_failures(agent_id, health_status).await
    }

    /// Get recovery statistics
    pub async fn get_statistics(&self) -> RecoveryStatistics {
        let stats = self.stats.read().await;
        let circuit_breaker_count = {
            let breakers = self.circuit_breakers.read().await;
            breakers.len()
        };

        RecoveryStatistics {
            total_failures: stats.total_failures,
            successful_recoveries: stats.successful_recoveries,
            failed_recoveries: stats.failed_recoveries,
            circuit_breaker_activations: stats.circuit_breaker_activations,
            active_circuit_breakers: circuit_breaker_count,
            recovery_success_rate: if stats.total_failures > 0 {
                stats.successful_recoveries as f64 / stats.total_failures as f64
            } else {
                0.0
            },
        }
    }

    /// Get all circuit breaker states
    pub async fn get_all_circuit_breaker_states(&self) -> HashMap<String, CircuitBreakerState> {
        let breakers = self.circuit_breakers.read().await;
        breakers
            .iter()
            .map(|(id, breaker)| (id.clone(), breaker.get_state()))
            .collect()
    }
}

impl Default for FailureRecoveryManager {
    fn default() -> Self {
        Self::new_with_defaults()
    }
}

/// Recovery statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStatistics {
    pub total_failures: u64,
    pub successful_recoveries: u64,
    pub failed_recoveries: u64,
    pub circuit_breaker_activations: u64,
    pub active_circuit_breakers: usize,
    pub recovery_success_rate: f64,
}