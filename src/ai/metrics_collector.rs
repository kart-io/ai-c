//! Agent metrics collection and monitoring system
//!
//! Provides comprehensive metrics collection, aggregation, and export
//! capabilities for the AI agent system with pluggable exporters.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info};

use super::AgentMetrics;

/// Metrics collection error types
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum MetricsError {
    #[error("Metric not found: {0}")]
    NotFound(String),

    #[error("Invalid metric value: {0}")]
    InvalidValue(String),

    #[error("Export failed: {0}")]
    ExportFailed(String),

    #[error("Registry error: {0}")]
    RegistryError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),
}

impl From<serde_json::Error> for MetricsError {
    fn from(err: serde_json::Error) -> Self {
        MetricsError::SerializationError(err.to_string())
    }
}

/// System-wide metrics
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemMetrics {
    /// Total number of requests processed
    pub total_requests: u64,
    /// Number of successful requests
    pub successful_requests: u64,
    /// Number of failed requests
    pub failed_requests: u64,
    /// Average response time across all agents
    pub average_response_time: Duration,
    /// Number of currently active agents
    pub active_agents: usize,
    /// System uptime since start
    pub system_uptime: Duration,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// CPU usage percentage
    pub cpu_usage: f64,
}

/// Individual metric types that can be collected
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Metric {
    /// Counter metric (monotonically increasing)
    Counter {
        name: String,
        value: u64,
        labels: HashMap<String, String>,
        timestamp: DateTime<Utc>,
    },
    /// Gauge metric (can increase or decrease)
    Gauge {
        name: String,
        value: f64,
        labels: HashMap<String, String>,
        timestamp: DateTime<Utc>,
    },
    /// Histogram metric for measuring distributions
    Histogram {
        name: String,
        value: f64,
        labels: HashMap<String, String>,
        timestamp: DateTime<Utc>,
    },
    /// Timer metric for measuring durations
    Timer {
        name: String,
        duration: Duration,
        labels: HashMap<String, String>,
        timestamp: DateTime<Utc>,
    },
}

impl Metric {
    /// Get metric name
    pub fn name(&self) -> &str {
        match self {
            Metric::Counter { name, .. } => name,
            Metric::Gauge { name, .. } => name,
            Metric::Histogram { name, .. } => name,
            Metric::Timer { name, .. } => name,
        }
    }

    /// Get metric labels
    pub fn labels(&self) -> &HashMap<String, String> {
        match self {
            Metric::Counter { labels, .. } => labels,
            Metric::Gauge { labels, .. } => labels,
            Metric::Histogram { labels, .. } => labels,
            Metric::Timer { labels, .. } => labels,
        }
    }

    /// Get metric timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Metric::Counter { timestamp, .. } => *timestamp,
            Metric::Gauge { timestamp, .. } => *timestamp,
            Metric::Histogram { timestamp, .. } => *timestamp,
            Metric::Timer { timestamp, .. } => *timestamp,
        }
    }

    /// Create a counter metric
    pub fn counter(name: impl Into<String>, value: u64, labels: HashMap<String, String>) -> Self {
        Self::Counter {
            name: name.into(),
            value,
            labels,
            timestamp: Utc::now(),
        }
    }

    /// Create a gauge metric
    pub fn gauge(name: impl Into<String>, value: f64, labels: HashMap<String, String>) -> Self {
        Self::Gauge {
            name: name.into(),
            value,
            labels,
            timestamp: Utc::now(),
        }
    }

    /// Create a histogram metric
    pub fn histogram(name: impl Into<String>, value: f64, labels: HashMap<String, String>) -> Self {
        Self::Histogram {
            name: name.into(),
            value,
            labels,
            timestamp: Utc::now(),
        }
    }

    /// Create a timer metric
    pub fn timer(name: impl Into<String>, duration: Duration, labels: HashMap<String, String>) -> Self {
        Self::Timer {
            name: name.into(),
            duration,
            labels,
            timestamp: Utc::now(),
        }
    }
}

/// Metrics registry for storing and managing metrics
pub struct MetricsRegistry {
    /// Stored metrics by name and labels
    metrics: Arc<RwLock<HashMap<String, Vec<Metric>>>>,
    /// Metric counters for efficient access
    counters: Arc<RwLock<HashMap<String, AtomicU64>>>,
    /// Metric gauges for efficient access
    gauges: Arc<RwLock<HashMap<String, Arc<RwLock<f64>>>>>,
}

impl MetricsRegistry {
    /// Create new metrics registry
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(HashMap::new())),
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register a metric
    pub async fn register_metric(&self, metric: Metric) -> Result<(), MetricsError> {
        let key = self.metric_key(&metric);

        {
            let mut metrics = self.metrics.write().await;
            metrics.entry(key.clone()).or_insert_with(Vec::new).push(metric.clone());
        }

        // Update efficient counters/gauges for frequently accessed metrics
        match metric {
            Metric::Counter { name, value, .. } => {
                let mut counters = self.counters.write().await;
                if let Some(counter) = counters.get(&name) {
                    counter.store(value, Ordering::Relaxed);
                } else {
                    counters.insert(name, AtomicU64::new(value));
                }
            }
            Metric::Gauge { name, value, .. } => {
                let mut gauges = self.gauges.write().await;
                if let Some(gauge) = gauges.get(&name) {
                    *gauge.write().await = value;
                } else {
                    gauges.insert(name, Arc::new(RwLock::new(value)));
                }
            }
            _ => {} // Histograms and timers are stored in main metrics map
        }

        debug!("Registered metric: {}", key);
        Ok(())
    }

    /// Get all metrics for a specific metric name
    pub async fn get_metrics(&self, name: &str) -> Vec<Metric> {
        let metrics = self.metrics.read().await;
        metrics
            .iter()
            .filter_map(|(key, metric_list)| {
                if key.starts_with(name) {
                    Some(metric_list.clone())
                } else {
                    None
                }
            })
            .flatten()
            .collect()
    }

    /// Get all metrics
    pub async fn get_all_metrics(&self) -> HashMap<String, Vec<Metric>> {
        self.metrics.read().await.clone()
    }

    /// Get counter value
    pub async fn get_counter(&self, name: &str) -> Option<u64> {
        let counters = self.counters.read().await;
        counters.get(name).map(|counter| counter.load(Ordering::Relaxed))
    }

    /// Get gauge value
    pub async fn get_gauge(&self, name: &str) -> Option<f64> {
        let gauges = self.gauges.read().await;
        if let Some(gauge) = gauges.get(name) {
            Some(*gauge.read().await)
        } else {
            None
        }
    }

    /// Clear old metrics (retention policy)
    pub async fn cleanup_old_metrics(&self, retention_period: Duration) {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(retention_period).unwrap_or_else(|_| chrono::Duration::zero());
        let mut metrics = self.metrics.write().await;

        for (_, metric_list) in metrics.iter_mut() {
            metric_list.retain(|metric| metric.timestamp() > cutoff_time);
        }

        // Remove empty metric lists
        metrics.retain(|_, metric_list| !metric_list.is_empty());

        debug!("Cleaned up metrics older than {:?}", retention_period);
    }

    /// Generate metric key from metric name and labels
    fn metric_key(&self, metric: &Metric) -> String {
        let mut key = metric.name().to_string();
        let mut labels: Vec<_> = metric.labels().iter().collect();
        labels.sort_by_key(|(k, _)| *k);

        if !labels.is_empty() {
            key.push('{');
            for (i, (k, v)) in labels.iter().enumerate() {
                if i > 0 {
                    key.push(',');
                }
                key.push_str(&format!("{}={}", k, v));
            }
            key.push('}');
        }

        key
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Metrics exporter trait for different export formats/destinations
#[async_trait]
pub trait MetricsExporter: Send + Sync {
    /// Export metrics to the configured destination
    async fn export(&self, metrics: &HashMap<String, Vec<Metric>>) -> Result<(), MetricsError>;

    /// Get exporter name
    fn name(&self) -> &str;
}

/// Prometheus format metrics exporter
pub struct PrometheusExporter {
    endpoint: String,
    client: reqwest::Client,
}

impl PrometheusExporter {
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            client: reqwest::Client::new(),
        }
    }

    fn format_prometheus_metrics(&self, metrics: &HashMap<String, Vec<Metric>>) -> String {
        let mut output = String::new();

        for (_, metric_list) in metrics {
            for metric in metric_list {
                match metric {
                    Metric::Counter { name, value, labels, .. } => {
                        output.push_str(&format!("# TYPE {} counter\n", name));
                        output.push_str(&format!("{}{} {}\n", name, self.format_labels(labels), value));
                    }
                    Metric::Gauge { name, value, labels, .. } => {
                        output.push_str(&format!("# TYPE {} gauge\n", name));
                        output.push_str(&format!("{}{} {}\n", name, self.format_labels(labels), value));
                    }
                    Metric::Histogram { name, value, labels, .. } => {
                        output.push_str(&format!("# TYPE {} histogram\n", name));
                        output.push_str(&format!("{}_bucket{} {}\n", name, self.format_labels(labels), value));
                    }
                    Metric::Timer { name, duration, labels, .. } => {
                        output.push_str(&format!("# TYPE {} histogram\n", name));
                        output.push_str(&format!("{}{} {}\n", name, self.format_labels(labels), duration.as_secs_f64()));
                    }
                }
            }
        }

        output
    }

    fn format_labels(&self, labels: &HashMap<String, String>) -> String {
        if labels.is_empty() {
            return String::new();
        }

        let mut formatted = String::from("{");
        let mut first = true;

        for (key, value) in labels {
            if !first {
                formatted.push(',');
            }
            formatted.push_str(&format!("{}=\"{}\"", key, value));
            first = false;
        }

        formatted.push('}');
        formatted
    }
}

#[async_trait]
impl MetricsExporter for PrometheusExporter {
    async fn export(&self, metrics: &HashMap<String, Vec<Metric>>) -> Result<(), MetricsError> {
        let prometheus_data = self.format_prometheus_metrics(metrics);

        match self.client.post(&self.endpoint)
            .header("Content-Type", "text/plain")
            .body(prometheus_data)
            .send()
            .await
        {
            Ok(response) => {
                if response.status().is_success() {
                    debug!("Successfully exported metrics to Prometheus");
                    Ok(())
                } else {
                    Err(MetricsError::ExportFailed(format!("HTTP error: {}", response.status())))
                }
            }
            Err(e) => Err(MetricsError::ExportFailed(format!("Request failed: {}", e))),
        }
    }

    fn name(&self) -> &str {
        "prometheus"
    }
}

/// JSON file metrics exporter
pub struct JsonFileExporter {
    file_path: std::path::PathBuf,
}

impl JsonFileExporter {
    pub fn new(file_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            file_path: file_path.into(),
        }
    }
}

#[async_trait]
impl MetricsExporter for JsonFileExporter {
    async fn export(&self, metrics: &HashMap<String, Vec<Metric>>) -> Result<(), MetricsError> {
        let json_data = serde_json::to_string_pretty(metrics)?;

        tokio::fs::write(&self.file_path, json_data)
            .await
            .map_err(|e| MetricsError::ExportFailed(format!("Failed to write file: {}", e)))?;

        debug!("Successfully exported metrics to file: {:?}", self.file_path);
        Ok(())
    }

    fn name(&self) -> &str {
        "json_file"
    }
}

/// Metrics collector configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Collection interval for automatic metrics gathering
    pub collection_interval: Duration,
    /// Retention period for stored metrics
    pub retention_period: Duration,
    /// Maximum number of metrics to store per type
    pub max_metrics_per_type: usize,
    /// Export interval for configured exporters
    pub export_interval: Duration,
    /// Buffer size for metrics collection
    pub buffer_size: usize,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(10),
            retention_period: Duration::from_secs(3600), // 1 hour
            max_metrics_per_type: 1000,
            export_interval: Duration::from_secs(60),
            buffer_size: 10000,
        }
    }
}

/// Main metrics collector
pub struct MetricsCollector {
    /// Metrics registry for storing metrics
    metrics_registry: Arc<MetricsRegistry>,
    /// Configured exporters
    exporters: Vec<Box<dyn MetricsExporter>>,
    /// Collection configuration
    config: MetricsConfig,
    /// System metrics tracking
    system_metrics: Arc<RwLock<SystemMetrics>>,
    /// Metrics collection channel
    metrics_channel: mpsc::UnboundedSender<Metric>,
    /// Background task handles
    _background_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl MetricsCollector {
    /// Create new metrics collector
    pub fn new(config: MetricsConfig) -> Self {
        let registry = Arc::new(MetricsRegistry::new());
        let system_metrics = Arc::new(RwLock::new(SystemMetrics::default()));
        let (metrics_tx, metrics_rx) = mpsc::unbounded_channel();

        let mut collector = Self {
            metrics_registry: registry.clone(),
            exporters: Vec::new(),
            config: config.clone(),
            system_metrics: system_metrics.clone(),
            metrics_channel: metrics_tx,
            _background_tasks: Vec::new(),
        };

        // Start background tasks
        collector.start_background_tasks(registry, system_metrics, metrics_rx, config);
        collector
    }

    /// Add metrics exporter
    pub fn add_exporter(&mut self, exporter: Box<dyn MetricsExporter>) {
        info!("Added metrics exporter: {}", exporter.name());
        self.exporters.push(exporter);
    }

    /// Record a metric
    pub fn record_metric(&self, metric: Metric) {
        if let Err(e) = self.metrics_channel.send(metric) {
            error!("Failed to send metric to collector: {}", e);
        }
    }

    /// Get agent metrics
    pub async fn get_agent_metrics(&self, agent_id: &str) -> Result<AgentMetrics, MetricsError> {
        let all_metrics = self.metrics_registry.get_all_metrics().await;

        let mut task_processing_count = 0;
        let mut total_response_time = Duration::default();
        let mut response_time_count = 0;
        let mut error_count = 0;

        for (_, metrics) in &all_metrics {
            for metric in metrics {
                if let Some(agent_label) = metric.labels().get("agent_id") {
                    if agent_label == agent_id {
                        match metric {
                            Metric::Counter { name, value, .. } => {
                                if name == "tasks_processed" {
                                    task_processing_count = *value;
                                } else if name == "errors" {
                                    error_count = *value;
                                }
                            }
                            Metric::Timer { name, duration, .. } => {
                                if name == "task_execution_time" {
                                    total_response_time += *duration;
                                    response_time_count += 1;
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        let average_response_time = if response_time_count > 0 {
            total_response_time / response_time_count as u32
        } else {
            Duration::default()
        };

        let error_rate = if task_processing_count > 0 {
            error_count as f64 / task_processing_count as f64
        } else {
            0.0
        };

        Ok(AgentMetrics {
            tasks_processed: task_processing_count,
            average_response_time,
            error_rate,
            last_activity: Utc::now(),
            memory_usage: 0, // TODO: Implement actual memory tracking
            cpu_usage: 0.0,  // TODO: Implement actual CPU tracking
        })
    }

    /// Get system metrics
    pub async fn get_system_metrics(&self) -> SystemMetrics {
        self.system_metrics.read().await.clone()
    }

    /// Export all metrics using configured exporters
    pub async fn export_metrics(&self) -> Result<(), MetricsError> {
        let metrics = self.metrics_registry.get_all_metrics().await;

        for exporter in &self.exporters {
            if let Err(e) = exporter.export(&metrics).await {
                error!("Export failed for {}: {}", exporter.name(), e);
            }
        }

        Ok(())
    }

    /// Start background collection and export tasks
    fn start_background_tasks(
        &mut self,
        registry: Arc<MetricsRegistry>,
        system_metrics: Arc<RwLock<SystemMetrics>>,
        mut metrics_rx: mpsc::UnboundedReceiver<Metric>,
        config: MetricsConfig,
    ) {
        // Metrics collection task
        let registry_clone = registry.clone();
        let collection_task = tokio::spawn(async move {
            while let Some(metric) = metrics_rx.recv().await {
                if let Err(e) = registry_clone.register_metric(metric).await {
                    error!("Failed to register metric: {}", e);
                }
            }
        });

        // Cleanup task
        let registry_clone = registry.clone();
        let retention_period = config.retention_period;
        let cleanup_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Clean every 5 minutes

            loop {
                interval.tick().await;
                registry_clone.cleanup_old_metrics(retention_period).await;
            }
        });

        // System metrics collection task
        let system_metrics_clone = system_metrics.clone();
        let system_start_time = std::time::Instant::now();
        let system_task = tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));

            loop {
                interval.tick().await;

                let mut metrics = system_metrics_clone.write().await;
                metrics.system_uptime = system_start_time.elapsed();

                // In a real implementation, you would collect actual system metrics here
                // For now, we'll just update the uptime
                debug!("Updated system metrics");
            }
        });

        self._background_tasks.push(collection_task);
        self._background_tasks.push(cleanup_task);
        self._background_tasks.push(system_task);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[tokio::test]
    async fn test_metrics_registry() {
        let registry = MetricsRegistry::new();

        let metric = Metric::counter("test_counter", 42, HashMap::new());
        registry.register_metric(metric).await.unwrap();

        let counter_value = registry.get_counter("test_counter").await;
        assert_eq!(counter_value, Some(42));
    }

    #[tokio::test]
    async fn test_metric_creation() {
        let labels = [("agent_id".to_string(), "test-agent".to_string())].into();

        let counter = Metric::counter("test_counter", 10, labels.clone());
        assert_eq!(counter.name(), "test_counter");

        let gauge = Metric::gauge("test_gauge", 3.14, labels.clone());
        assert_eq!(gauge.name(), "test_gauge");

        let timer = Metric::timer("test_timer", Duration::from_millis(100), labels);
        assert_eq!(timer.name(), "test_timer");
    }

    #[tokio::test]
    async fn test_json_file_exporter() {
        let temp_file = NamedTempFile::new().unwrap();
        let exporter = JsonFileExporter::new(temp_file.path());

        let mut metrics = HashMap::new();
        metrics.insert(
            "test_metric".to_string(),
            vec![Metric::counter("test_counter", 42, HashMap::new())]
        );

        exporter.export(&metrics).await.unwrap();

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("test_counter"));
        assert!(content.contains("42"));
    }

    #[tokio::test]
    async fn test_metrics_collector() {
        let config = MetricsConfig::default();
        let mut collector = MetricsCollector::new(config);

        let temp_file = NamedTempFile::new().unwrap();
        collector.add_exporter(Box::new(JsonFileExporter::new(temp_file.path())));

        let metric = Metric::counter("test_counter", 100, HashMap::new());
        collector.record_metric(metric);

        // Give some time for async processing
        tokio::time::sleep(Duration::from_millis(100)).await;

        collector.export_metrics().await.unwrap();

        let content = tokio::fs::read_to_string(temp_file.path()).await.unwrap();
        assert!(content.contains("test_counter"));
    }

    #[tokio::test]
    async fn test_prometheus_formatting() {
        let exporter = PrometheusExporter::new("http://localhost:9090/metrics");

        let mut metrics = HashMap::new();
        let labels = [("agent_id".to_string(), "test-agent".to_string())].into();
        metrics.insert(
            "test_counter".to_string(),
            vec![Metric::counter("test_counter", 42, labels)]
        );

        let formatted = exporter.format_prometheus_metrics(&metrics);
        assert!(formatted.contains("# TYPE test_counter counter"));
        assert!(formatted.contains("test_counter{agent_id=\"test-agent\"} 42"));
    }
}