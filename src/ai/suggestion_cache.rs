//! AI Suggestion Cache System
//!
//! Provides intelligent caching for AI-generated suggestions with:
//! - LRU eviction policy for memory management
//! - TTL-based expiration for freshness
//! - Intelligent cache key generation
//! - Performance metrics and monitoring

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::Duration,
};
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

use crate::error::{AppError, AppResult};

/// Cache entry for AI suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    /// Cached value
    pub value: T,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Expiration time
    pub expires_at: DateTime<Utc>,
    /// Access count for popularity tracking
    pub access_count: u64,
    /// Last access time
    pub last_accessed: DateTime<Utc>,
    /// Cache hit metadata
    pub metadata: HashMap<String, String>,
}

impl<T> CacheEntry<T> {
    /// Create a new cache entry with TTL
    pub fn new(value: T, ttl: Duration) -> Self {
        let now = Utc::now();
        let expires_at = now + chrono::Duration::from_std(ttl).unwrap_or_default();

        Self {
            value,
            created_at: now,
            expires_at,
            access_count: 0,
            last_accessed: now,
            metadata: HashMap::new(),
        }
    }

    /// Check if the entry has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Mark entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_accessed = Utc::now();
    }

    /// Get age of the entry
    pub fn age(&self) -> chrono::Duration {
        Utc::now() - self.created_at
    }
}

/// LRU cache node for efficient eviction
#[derive(Debug)]
struct LRUNode {
    key: String,
    prev: Option<String>,
    next: Option<String>,
}

/// Configuration for suggestion cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SuggestionCacheConfig {
    /// Maximum number of entries
    pub max_entries: usize,
    /// Default TTL for cache entries
    pub default_ttl: Duration,
    /// Cleanup interval for expired entries
    pub cleanup_interval: Duration,
    /// Enable cache compression
    pub enable_compression: bool,
    /// Cache metrics collection
    pub enable_metrics: bool,
}

impl Default for SuggestionCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 10000,
            default_ttl: Duration::from_secs(3600), // 1 hour
            cleanup_interval: Duration::from_secs(300), // 5 minutes
            enable_compression: false,
            enable_metrics: true,
        }
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Total number of entries
    pub total_entries: u64,
    /// Total expired entries cleaned up
    pub expired_cleaned: u64,
    /// Total evicted entries (LRU)
    pub evicted_entries: u64,
    /// Cache memory usage estimate (bytes)
    pub memory_usage: u64,
}

impl CacheStats {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        if self.hits + self.misses == 0 {
            0.0
        } else {
            self.hits as f64 / (self.hits + self.misses) as f64
        }
    }

    /// Reset statistics
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// AI Suggestion Cache with LRU eviction and TTL expiration
pub struct SuggestionCache<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Cache storage
    cache: Arc<RwLock<HashMap<String, CacheEntry<T>>>>,
    /// LRU tracking for eviction
    lru_order: Arc<RwLock<HashMap<String, LRUNode>>>,
    /// Most recently used key
    mru_key: Arc<RwLock<Option<String>>>,
    /// Least recently used key
    lru_key: Arc<RwLock<Option<String>>>,
    /// Cache configuration
    config: SuggestionCacheConfig,
    /// Cache statistics
    stats: Arc<RwLock<CacheStats>>,
}

impl<T> SuggestionCache<T>
where
    T: Clone + Send + Sync + 'static,
{
    /// Create a new suggestion cache
    pub fn new(config: SuggestionCacheConfig) -> Self {
        Self {
            cache: Arc::new(RwLock::new(HashMap::new())),
            lru_order: Arc::new(RwLock::new(HashMap::new())),
            mru_key: Arc::new(RwLock::new(None)),
            lru_key: Arc::new(RwLock::new(None)),
            config,
            stats: Arc::new(RwLock::new(CacheStats::default())),
        }
    }

    /// Get value from cache
    pub async fn get(&self, key: &str) -> Option<T> {
        let mut cache = self.cache.write().await;
        let mut stats = if self.config.enable_metrics {
            Some(self.stats.write().await)
        } else {
            None
        };

        if let Some(entry) = cache.get_mut(key) {
            // Check if entry has expired
            if entry.is_expired() {
                cache.remove(key);
                self.remove_from_lru(key).await;
                if let Some(ref mut stats) = stats {
                    stats.misses += 1;
                    stats.expired_cleaned += 1;
                }
                return None;
            }

            // Update access info
            entry.mark_accessed();
            self.move_to_front(key).await;

            if let Some(ref mut stats) = stats {
                stats.hits += 1;
            }

            debug!("Cache hit for key: {}", key);
            Some(entry.value.clone())
        } else {
            if let Some(ref mut stats) = stats {
                stats.misses += 1;
            }
            debug!("Cache miss for key: {}", key);
            None
        }
    }

    /// Put value into cache
    pub async fn put(&self, key: String, value: T) -> AppResult<()> {
        self.put_with_ttl(key, value, self.config.default_ttl).await
    }

    /// Insert value into cache (alias for put)
    pub async fn insert(&self, key: String, value: T) -> AppResult<()> {
        self.put(key, value).await
    }

    /// Put value into cache with custom TTL
    pub async fn put_with_ttl(&self, key: String, value: T, ttl: Duration) -> AppResult<()> {
        let entry = CacheEntry::new(value, ttl);
        let mut cache = self.cache.write().await;

        // Check if we need to evict entries
        if cache.len() >= self.config.max_entries && !cache.contains_key(&key) {
            self.evict_lru().await?;
        }

        // Insert or update entry
        let is_new = !cache.contains_key(&key);
        cache.insert(key.clone(), entry);

        // Update LRU tracking
        if is_new {
            self.add_to_front(&key).await;
            if self.config.enable_metrics {
                let mut stats = self.stats.write().await;
                stats.total_entries += 1;
            }
        } else {
            self.move_to_front(&key).await;
        }

        debug!("Cached entry for key: {}", key);
        Ok(())
    }

    /// Remove entry from cache
    pub async fn remove(&self, key: &str) -> Option<T> {
        let mut cache = self.cache.write().await;
        if let Some(entry) = cache.remove(key) {
            self.remove_from_lru(key).await;
            if self.config.enable_metrics {
                let mut stats = self.stats.write().await;
                stats.total_entries = stats.total_entries.saturating_sub(1);
            }
            debug!("Removed cache entry for key: {}", key);
            Some(entry.value)
        } else {
            None
        }
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();

        let mut lru_order = self.lru_order.write().await;
        lru_order.clear();

        *self.mru_key.write().await = None;
        *self.lru_key.write().await = None;

        if self.config.enable_metrics {
            let mut stats = self.stats.write().await;
            stats.total_entries = 0;
        }

        info!("Cleared all cache entries");
    }

    /// Get cache size
    pub async fn size(&self) -> usize {
        self.cache.read().await.len()
    }

    /// Check if cache contains key
    pub async fn contains_key(&self, key: &str) -> bool {
        let cache = self.cache.read().await;
        if let Some(entry) = cache.get(key) {
            !entry.is_expired()
        } else {
            false
        }
    }

    /// Cleanup expired entries
    pub async fn cleanup_expired(&self) -> usize {
        let mut cache = self.cache.write().await;
        let now = Utc::now();
        let mut expired_keys = Vec::new();

        for (key, entry) in cache.iter() {
            if now > entry.expires_at {
                expired_keys.push(key.clone());
            }
        }

        let expired_count = expired_keys.len();
        for key in expired_keys {
            cache.remove(&key);
            self.remove_from_lru(&key).await;
        }

        if self.config.enable_metrics && expired_count > 0 {
            let mut stats = self.stats.write().await;
            stats.expired_cleaned += expired_count as u64;
            stats.total_entries = stats.total_entries.saturating_sub(expired_count as u64);
        }

        if expired_count > 0 {
            info!("Cleaned up {} expired cache entries", expired_count);
        }

        expired_count
    }

    /// Get cache statistics
    pub async fn stats(&self) -> CacheStats {
        if self.config.enable_metrics {
            self.stats.read().await.clone()
        } else {
            CacheStats::default()
        }
    }

    /// Generate cache key for commit suggestions
    pub fn commit_suggestion_key(
        files: &[String],
        diff_content: &str,
        style: &str,
        context: Option<&str>,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        files.hash(&mut hasher);
        diff_content.hash(&mut hasher);
        style.hash(&mut hasher);
        context.hash(&mut hasher);

        format!("commit_{}_{}", style, hasher.finish())
    }

    /// Generate cache key for code analysis
    pub fn code_analysis_key(file_paths: &[String], analysis_type: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        file_paths.hash(&mut hasher);
        analysis_type.hash(&mut hasher);

        format!("analysis_{}_{}", analysis_type, hasher.finish())
    }

    /// Evict least recently used entry
    async fn evict_lru(&self) -> AppResult<()> {
        let lru_key = {
            let lru_key_guard = self.lru_key.read().await;
            lru_key_guard.clone()
        };

        if let Some(key) = lru_key {
            self.remove(&key).await;
            if self.config.enable_metrics {
                let mut stats = self.stats.write().await;
                stats.evicted_entries += 1;
            }
            debug!("Evicted LRU entry: {}", key);
        }

        Ok(())
    }

    /// Add key to front of LRU list
    async fn add_to_front(&self, key: &str) {
        let mut lru_order = self.lru_order.write().await;
        let mut mru_key = self.mru_key.write().await;
        let mut lru_key = self.lru_key.write().await;

        let node = LRUNode {
            key: key.to_string(),
            prev: None,
            next: mru_key.clone(),
        };

        if let Some(ref old_mru) = *mru_key {
            if let Some(old_mru_node) = lru_order.get_mut(old_mru) {
                old_mru_node.prev = Some(key.to_string());
            }
        } else {
            // First entry
            *lru_key = Some(key.to_string());
        }

        lru_order.insert(key.to_string(), node);
        *mru_key = Some(key.to_string());
    }

    /// Move key to front of LRU list
    async fn move_to_front(&self, key: &str) {
        self.remove_from_lru(key).await;
        self.add_to_front(key).await;
    }

    /// Remove key from LRU list
    async fn remove_from_lru(&self, key: &str) {
        let mut lru_order = self.lru_order.write().await;
        let mut mru_key = self.mru_key.write().await;
        let mut lru_key = self.lru_key.write().await;

        if let Some(node) = lru_order.remove(key) {
            // Update previous node's next pointer
            if let Some(ref prev_key) = node.prev {
                if let Some(prev_node) = lru_order.get_mut(prev_key) {
                    prev_node.next = node.next.clone();
                }
            } else {
                // This was the MRU node
                *mru_key = node.next.clone();
            }

            // Update next node's prev pointer
            if let Some(ref next_key) = node.next {
                if let Some(next_node) = lru_order.get_mut(next_key) {
                    next_node.prev = node.prev.clone();
                }
            } else {
                // This was the LRU node
                *lru_key = node.prev.clone();
            }
        }
    }

    /// Start background cleanup task
    pub fn start_cleanup_task(cache: Arc<Self>) -> tokio::task::JoinHandle<()> {
        let cleanup_interval = cache.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;
                let expired_count = cache.cleanup_expired().await;

                if expired_count > 0 {
                    debug!("Background cleanup removed {} expired entries", expired_count);
                }
            }
        })
    }
}

/// Specialized cache for commit suggestions
pub type CommitSuggestionCache = SuggestionCache<Vec<String>>;

/// Specialized cache for code analysis results
pub type CodeAnalysisCache = SuggestionCache<serde_json::Value>;

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration as TokioDuration};

    #[tokio::test]
    async fn test_cache_basic_operations() {
        let config = SuggestionCacheConfig::default();
        let cache = SuggestionCache::<String>::new(config);

        // Test put and get
        cache.put("key1".to_string(), "value1".to_string()).await.unwrap();
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));

        // Test cache miss
        assert_eq!(cache.get("nonexistent").await, None);

        // Test size
        assert_eq!(cache.size().await, 1);

        // Test contains_key
        assert!(cache.contains_key("key1").await);
        assert!(!cache.contains_key("key2").await);
    }

    #[tokio::test]
    async fn test_cache_ttl_expiration() {
        let config = SuggestionCacheConfig {
            default_ttl: Duration::from_millis(100),
            ..Default::default()
        };
        let cache = SuggestionCache::<String>::new(config);

        // Put entry with short TTL
        cache.put("key1".to_string(), "value1".to_string()).await.unwrap();
        assert_eq!(cache.get("key1").await, Some("value1".to_string()));

        // Wait for expiration
        sleep(TokioDuration::from_millis(150)).await;
        assert_eq!(cache.get("key1").await, None);
    }

    #[tokio::test]
    async fn test_cache_lru_eviction() {
        let config = SuggestionCacheConfig {
            max_entries: 2,
            ..Default::default()
        };
        let cache = SuggestionCache::<String>::new(config);

        // Fill cache to capacity
        cache.put("key1".to_string(), "value1".to_string()).await.unwrap();
        cache.put("key2".to_string(), "value2".to_string()).await.unwrap();

        // Verify both entries exist
        assert!(cache.contains_key("key1").await);
        assert!(cache.contains_key("key2").await);

        // Access key1 to make it MRU
        cache.get("key1").await;

        // Add third entry, should evict key2 (LRU)
        cache.put("key3".to_string(), "value3".to_string()).await.unwrap();

        assert!(cache.contains_key("key1").await);
        assert!(!cache.contains_key("key2").await);
        assert!(cache.contains_key("key3").await);
    }

    #[tokio::test]
    async fn test_cache_cleanup_expired() {
        let config = SuggestionCacheConfig {
            default_ttl: Duration::from_millis(100),
            ..Default::default()
        };
        let cache = SuggestionCache::<String>::new(config);

        // Add entries
        cache.put("key1".to_string(), "value1".to_string()).await.unwrap();
        cache.put("key2".to_string(), "value2".to_string()).await.unwrap();

        // Wait for expiration
        sleep(TokioDuration::from_millis(150)).await;

        // Cleanup expired entries
        let cleaned = cache.cleanup_expired().await;
        assert_eq!(cleaned, 2);
        assert_eq!(cache.size().await, 0);
    }

    #[tokio::test]
    async fn test_cache_statistics() {
        let config = SuggestionCacheConfig {
            enable_metrics: true,
            ..Default::default()
        };
        let cache = SuggestionCache::<String>::new(config);

        // Test cache hits and misses
        cache.get("nonexistent").await; // miss
        cache.put("key1".to_string(), "value1".to_string()).await.unwrap();
        cache.get("key1").await; // hit

        let stats = cache.stats().await;
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_entries, 1);
        assert!(stats.hit_rate() > 0.0);
    }

    #[tokio::test]
    async fn test_cache_key_generation() {
        let files = vec!["file1.rs".to_string(), "file2.rs".to_string()];
        let diff = "diff content";
        let style = "conventional";

        let key1 = SuggestionCache::<String>::commit_suggestion_key(&files, diff, style, None);
        let key2 = SuggestionCache::<String>::commit_suggestion_key(&files, diff, style, None);
        let key3 = SuggestionCache::<String>::commit_suggestion_key(&files, "different diff", style, None);

        // Same input should produce same key
        assert_eq!(key1, key2);
        // Different input should produce different key
        assert_ne!(key1, key3);
    }
}