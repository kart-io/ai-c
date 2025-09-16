//! Health monitoring system for agents
//!
//! Provides comprehensive health checking, monitoring, and alerting
//! for the agent system with configurable checks and thresholds.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use super::HealthStatus;

/// Health monitor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// Health check interval in seconds
    pub check_interval: Duration,
    /// Maximum failures before marking agent as unhealthy
    pub max_failures: u32,
    /// Recovery attempt interval in seconds
    pub recovery_interval: Duration,
    /// Enable automatic recovery attempts
    pub enable_auto_recovery: bool,
    /// Response timeout for health checks
    pub response_timeout: Duration,
    /// Maximum number of concurrent health checks
    pub max_concurrent_checks: usize,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            max_failures: 3,
            recovery_interval: Duration::from_secs(60),
            enable_auto_recovery: true,
            response_timeout: Duration::from_secs(5),
            max_concurrent_checks: 10,
        }
    }
}

/// Error types for health monitoring
#[derive(Debug, Clone, thiserror::Error)]
pub enum MonitorError {
    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Health check failed: {0}")]
    HealthCheckFailed(String),

    #[error("Timeout during health check: {0}")]
    Timeout(String),

    #[error("Too many concurrent checks")]
    TooManyConcurrentChecks,

    #[error("Monitoring configuration error: {0}")]
    ConfigError(String),

    #[error("Internal monitoring error: {0}")]
    InternalError(String),
}

/// Monitored agent information
#[derive(Debug, Clone)]
pub struct MonitoredAgent {
    /// Agent unique identifier
    pub agent_id: String,
    /// Last health check timestamp
    pub last_health_check: SystemTime,
    /// Current health status
    pub health_status: HealthStatus,
    /// Number of consecutive failures
    pub failure_count: u32,
    /// Number of recovery attempts made
    pub recovery_attempts: u32,
    /// Agent monitoring start time
    pub monitoring_started: SystemTime,
    /// Last successful check time
    pub last_successful_check: Option<SystemTime>,
}

impl MonitoredAgent {
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            last_health_check: SystemTime::now(),
            health_status: HealthStatus::Healthy,
            failure_count: 0,
            recovery_attempts: 0,
            monitoring_started: SystemTime::now(),
            last_successful_check: Some(SystemTime::now()),
        }
    }

    /// Check if agent needs recovery
    pub fn needs_recovery(&self, config: &HealthMonitorConfig) -> bool {
        self.failure_count >= config.max_failures
            && config.enable_auto_recovery
            && matches!(self.health_status, HealthStatus::Unhealthy(_))
    }

    /// Update after successful health check
    pub fn mark_healthy(&mut self) {
        self.health_status = HealthStatus::Healthy;
        self.failure_count = 0;
        self.last_health_check = SystemTime::now();
        self.last_successful_check = Some(SystemTime::now());
    }

    /// Update after failed health check
    pub fn mark_unhealthy(&mut self, error: String) {
        self.failure_count += 1;
        self.health_status = HealthStatus::Unhealthy(error);
        self.last_health_check = SystemTime::now();
    }

    /// Update after degraded health check
    pub fn mark_degraded(&mut self, warning: String) {
        self.health_status = HealthStatus::Degraded(warning);
        self.last_health_check = SystemTime::now();
        // Don't increment failure count for degraded state
    }
}

/// System health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall system health status
    pub system_health: HealthStatus,
    /// Total number of monitored agents
    pub total_agents: usize,
    /// List of healthy agent IDs
    pub healthy_agents: Vec<String>,
    /// List of unhealthy agent IDs
    pub unhealthy_agents: Vec<String>,
    /// List of degraded agent IDs
    pub degraded_agents: Vec<String>,
    /// Report generation timestamp
    pub generated_at: DateTime<Utc>,
    /// System uptime
    pub uptime: Duration,
    /// Total health checks performed
    pub total_checks_performed: u64,
    /// Average response time across all checks
    pub average_response_time: Duration,
}

/// Health check trait for pluggable health checks
#[async_trait]
pub trait HealthCheck: Send + Sync {
    /// Get the name of this health check
    fn name(&self) -> &str;

    /// Perform health check on the specified agent
    async fn check(&self, agent_id: &str) -> Result<HealthStatus, MonitorError>;

    /// Get check timeout
    fn timeout(&self) -> Duration {
        Duration::from_secs(5)
    }

    /// Check priority (higher numbers run first)
    fn priority(&self) -> u8 {
        100
    }
}

/// Response time health check
#[derive(Debug)]
pub struct ResponseTimeCheck {
    max_response_time: Duration,
    name: String,
}

impl ResponseTimeCheck {
    pub fn new(max_response_time: Duration) -> Self {
        Self {
            max_response_time,
            name: "ResponseTime".to_string(),
        }
    }
}

#[async_trait]
impl HealthCheck for ResponseTimeCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self, agent_id: &str) -> Result<HealthStatus, MonitorError> {
        let start = SystemTime::now();

        // Simulate ping-like health check
        tokio::time::sleep(Duration::from_millis(10)).await;

        let elapsed = start.elapsed().unwrap_or(Duration::from_secs(999));

        if elapsed > self.max_response_time {
            Ok(HealthStatus::Degraded(format!(
                "Response time {}ms exceeds threshold {}ms",
                elapsed.as_millis(),
                self.max_response_time.as_millis()
            )))
        } else {
            Ok(HealthStatus::Healthy)
        }
    }

    fn timeout(&self) -> Duration {
        self.max_response_time + Duration::from_secs(1)
    }
}

/// Memory usage health check
#[derive(Debug)]
pub struct MemoryUsageCheck {
    max_memory_mb: u64,
    name: String,
}

impl MemoryUsageCheck {
    pub fn new(max_memory_mb: u64) -> Self {
        Self {
            max_memory_mb,
            name: "MemoryUsage".to_string(),
        }
    }
}

#[async_trait]
impl HealthCheck for MemoryUsageCheck {
    fn name(&self) -> &str {
        &self.name
    }

    async fn check(&self, _agent_id: &str) -> Result<HealthStatus, MonitorError> {
        // Simplified memory check - in real implementation would check actual agent memory
        let simulated_memory_mb = 50; // Simulate current memory usage

        if simulated_memory_mb > self.max_memory_mb {
            Ok(HealthStatus::Unhealthy(format!(
                "Memory usage {}MB exceeds limit {}MB",
                simulated_memory_mb, self.max_memory_mb
            )))
        } else if simulated_memory_mb > (self.max_memory_mb * 80 / 100) {
            Ok(HealthStatus::Degraded(format!(
                "Memory usage {}MB approaching limit {}MB",
                simulated_memory_mb, self.max_memory_mb
            )))
        } else {
            Ok(HealthStatus::Healthy)
        }
    }
}

/// Alerting service trait for notifications
#[async_trait]
pub trait AlertingService: Send + Sync {
    /// Send health alert
    async fn send_alert(&self, agent_id: &str, status: &HealthStatus) -> Result<(), MonitorError>;
}

/// Simple logging alerting service
#[derive(Debug, Default)]
pub struct LoggingAlertingService;

#[async_trait]
impl AlertingService for LoggingAlertingService {
    async fn send_alert(&self, agent_id: &str, status: &HealthStatus) -> Result<(), MonitorError> {
        match status {
            HealthStatus::Unhealthy(msg) => {
                error!("ALERT: Agent {} is unhealthy: {}", agent_id, msg);
            }
            HealthStatus::Degraded(msg) => {
                warn!("WARNING: Agent {} is degraded: {}", agent_id, msg);
            }
            HealthStatus::Healthy => {
                info!("RECOVERY: Agent {} is now healthy", agent_id);
            }
            HealthStatus::Shutdown => {
                info!("INFO: Agent {} has shutdown", agent_id);
            }
        }
        Ok(())
    }
}

/// Health monitor implementation
pub struct HealthMonitor {
    /// Monitored agents
    monitored_agents: Arc<RwLock<HashMap<String, MonitoredAgent>>>,
    /// Registered health checks
    health_checks: Arc<RwLock<Vec<Arc<dyn HealthCheck>>>>,
    /// Alerting service
    alerting_service: Arc<dyn AlertingService>,
    /// Configuration
    config: HealthMonitorConfig,
    /// System startup time
    startup_time: SystemTime,
    /// Statistics
    stats: Arc<RwLock<HealthMonitorStats>>,
}

/// Health monitor statistics
#[derive(Debug, Default)]
struct HealthMonitorStats {
    total_checks_performed: u64,
    total_response_time: Duration,
    alerts_sent: u64,
    recovery_attempts: u64,
}

impl HealthMonitor {
    /// Create a new health monitor
    pub fn new(config: HealthMonitorConfig) -> Self {
        info!("Creating HealthMonitor with config: {:?}", config);

        Self {
            monitored_agents: Arc::new(RwLock::new(HashMap::new())),
            health_checks: Arc::new(RwLock::new(Vec::new())),
            alerting_service: Arc::new(LoggingAlertingService::default()),
            config,
            startup_time: SystemTime::now(),
            stats: Arc::new(RwLock::new(HealthMonitorStats::default())),
        }
    }

    /// Create with custom alerting service
    pub fn new_with_alerting(
        config: HealthMonitorConfig,
        alerting_service: Arc<dyn AlertingService>,
    ) -> Self {
        let mut monitor = Self::new(config);
        monitor.alerting_service = alerting_service;
        monitor
    }

    /// Start monitoring an agent
    pub async fn start_monitoring(&self, agent_id: String) -> Result<(), MonitorError> {
        info!("Starting monitoring for agent: {}", agent_id);

        let monitored_agent = MonitoredAgent::new(agent_id.clone());

        {
            let mut agents = self.monitored_agents.write().await;
            agents.insert(agent_id.clone(), monitored_agent);
        }

        debug!("Agent {} added to monitoring", agent_id);
        Ok(())
    }

    /// Stop monitoring an agent
    pub async fn stop_monitoring(&self, agent_id: &str) -> Result<(), MonitorError> {
        info!("Stopping monitoring for agent: {}", agent_id);

        {
            let mut agents = self.monitored_agents.write().await;
            agents.remove(agent_id);
        }

        debug!("Agent {} removed from monitoring", agent_id);
        Ok(())
    }

    /// Perform health check on a specific agent
    pub async fn perform_health_check(&self, agent_id: &str) -> Result<HealthStatus, MonitorError> {
        debug!("Performing health check for agent: {}", agent_id);

        // Check if agent is being monitored
        let agent_exists = {
            let agents = self.monitored_agents.read().await;
            agents.contains_key(agent_id)
        };

        if !agent_exists {
            return Err(MonitorError::AgentNotFound(agent_id.to_string()));
        }

        // Run all health checks
        let health_checks = {
            let checks = self.health_checks.read().await;
            checks.clone()
        };

        let start_time = SystemTime::now();
        let mut overall_status = HealthStatus::Healthy;
        let mut check_results = Vec::new();

        for check in health_checks.iter() {
            match tokio::time::timeout(check.timeout(), check.check(agent_id)).await {
                Ok(Ok(status)) => {
                    check_results.push((check.name(), status.clone()));

                    // Determine overall status (worst case wins)
                    match (&overall_status, &status) {
                        (_, HealthStatus::Unhealthy(_)) => overall_status = status,
                        (HealthStatus::Healthy, HealthStatus::Degraded(_)) => overall_status = status,
                        _ => {}
                    }
                }
                Ok(Err(e)) => {
                    warn!("Health check {} failed for agent {}: {}", check.name(), agent_id, e);
                    overall_status = HealthStatus::Unhealthy(format!("Check {} failed: {}", check.name(), e));
                }
                Err(_) => {
                    warn!("Health check {} timed out for agent {}", check.name(), agent_id);
                    overall_status = HealthStatus::Unhealthy(format!("Check {} timed out", check.name()));
                }
            }
        }

        let check_duration = start_time.elapsed().unwrap_or(Duration::ZERO);

        // Update agent status
        {
            let mut agents = self.monitored_agents.write().await;
            if let Some(agent) = agents.get_mut(agent_id) {
                let previous_status = agent.health_status.clone();

                match &overall_status {
                    HealthStatus::Healthy => agent.mark_healthy(),
                    HealthStatus::Degraded(msg) => agent.mark_degraded(msg.clone()),
                    HealthStatus::Unhealthy(msg) => agent.mark_unhealthy(msg.clone()),
                    HealthStatus::Shutdown => {
                        agent.health_status = HealthStatus::Shutdown;
                        agent.last_health_check = SystemTime::now();
                    }
                }

                // Send alert if status changed
                if !matches!((previous_status, &overall_status),
                    (HealthStatus::Healthy, HealthStatus::Healthy) |
                    (HealthStatus::Degraded(_), HealthStatus::Degraded(_)) |
                    (HealthStatus::Unhealthy(_), HealthStatus::Unhealthy(_)) |
                    (HealthStatus::Shutdown, HealthStatus::Shutdown)
                ) {
                    if let Err(e) = self.alerting_service.send_alert(agent_id, &overall_status).await {
                        warn!("Failed to send alert for agent {}: {}", agent_id, e);
                    } else {
                        let mut stats = self.stats.write().await;
                        stats.alerts_sent += 1;
                    }
                }
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.write().await;
            stats.total_checks_performed += 1;
            stats.total_response_time += check_duration;
        }

        debug!("Health check completed for agent {} in {:?}: {:?}",
               agent_id, check_duration, overall_status);

        Ok(overall_status)
    }

    /// Perform health checks on all monitored agents
    pub async fn perform_all_health_checks(&self) -> Result<(), MonitorError> {
        debug!("Performing health checks on all monitored agents");

        let agent_ids: Vec<String> = {
            let agents = self.monitored_agents.read().await;
            agents.keys().cloned().collect()
        };

        let mut results = Vec::new();
        for agent_id in agent_ids {
            let result = self.perform_health_check(&agent_id).await;
            results.push((agent_id, result));
        }

        // Log summary
        let healthy_count = results.iter()
            .filter(|(_, result)| matches!(result, Ok(HealthStatus::Healthy)))
            .count();
        let total_count = results.len();

        info!("Health check summary: {}/{} agents healthy", healthy_count, total_count);

        Ok(())
    }

    /// Get comprehensive health report
    pub async fn get_health_report(&self) -> HealthReport {
        let agents = self.monitored_agents.read().await;
        let stats = self.stats.read().await;

        let mut healthy_agents = Vec::new();
        let mut unhealthy_agents = Vec::new();
        let mut degraded_agents = Vec::new();

        let mut system_health = HealthStatus::Healthy;
        let mut unhealthy_count = 0;

        for (agent_id, agent) in agents.iter() {
            match &agent.health_status {
                HealthStatus::Healthy => healthy_agents.push(agent_id.clone()),
                HealthStatus::Unhealthy(_) => {
                    unhealthy_agents.push(agent_id.clone());
                    unhealthy_count += 1;
                }
                HealthStatus::Degraded(_) => degraded_agents.push(agent_id.clone()),
                HealthStatus::Shutdown => {} // Don't include shutdown agents
            }
        }

        // Determine overall system health
        if unhealthy_count > 0 {
            if unhealthy_count == agents.len() {
                system_health = HealthStatus::Unhealthy("All agents are unhealthy".to_string());
            } else {
                system_health = HealthStatus::Degraded(format!("{} of {} agents are unhealthy", unhealthy_count, agents.len()));
            }
        } else if !degraded_agents.is_empty() {
            system_health = HealthStatus::Degraded(format!("{} agents are degraded", degraded_agents.len()));
        }

        let average_response_time = if stats.total_checks_performed > 0 {
            stats.total_response_time / stats.total_checks_performed as u32
        } else {
            Duration::ZERO
        };

        HealthReport {
            system_health,
            total_agents: agents.len(),
            healthy_agents,
            unhealthy_agents,
            degraded_agents,
            generated_at: Utc::now(),
            uptime: self.startup_time.elapsed().unwrap_or(Duration::ZERO),
            total_checks_performed: stats.total_checks_performed,
            average_response_time,
        }
    }

    /// Register a health check
    pub async fn register_health_check(&self, check: Arc<dyn HealthCheck>) {
        info!("Registering health check: {}", check.name());

        let mut checks = self.health_checks.write().await;
        checks.push(check);

        // Sort by priority (higher priority first)
        checks.sort_by(|a, b| b.priority().cmp(&a.priority()));
    }

    /// Get list of monitored agents
    pub async fn get_monitored_agents(&self) -> Vec<String> {
        let agents = self.monitored_agents.read().await;
        agents.keys().cloned().collect()
    }

    /// Check if an agent is being monitored
    pub async fn is_monitoring(&self, agent_id: &str) -> bool {
        let agents = self.monitored_agents.read().await;
        agents.contains_key(agent_id)
    }

    /// Get health monitor statistics
    pub async fn get_statistics(&self) -> HealthMonitorStatistics {
        let stats = self.stats.read().await;
        let agents = self.monitored_agents.read().await;

        HealthMonitorStatistics {
            total_agents_monitored: agents.len(),
            total_checks_performed: stats.total_checks_performed,
            average_response_time: if stats.total_checks_performed > 0 {
                stats.total_response_time / stats.total_checks_performed as u32
            } else {
                Duration::ZERO
            },
            alerts_sent: stats.alerts_sent,
            recovery_attempts: stats.recovery_attempts,
            uptime: self.startup_time.elapsed().unwrap_or(Duration::ZERO),
        }
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(HealthMonitorConfig::default())
    }
}

/// Health monitor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorStatistics {
    pub total_agents_monitored: usize,
    pub total_checks_performed: u64,
    pub average_response_time: Duration,
    pub alerts_sent: u64,
    pub recovery_attempts: u64,
    pub uptime: Duration,
}
