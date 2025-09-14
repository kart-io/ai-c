//! Status caching system for Git operations
//!
//! Provides intelligent caching to optimize performance for large repositories
//! with >10,000 files while maintaining data freshness.

use crate::git::FileStatus;
use chrono::{DateTime, Utc};
use std::time::{Duration, SystemTime};

/// Status cache for Git file status operations
///
/// Uses LRU-style caching with time-based invalidation to balance
/// performance with data freshness requirements.
#[derive(Debug)]
pub struct StatusCache {
    /// Cached file status entries
    cached_status: Option<Vec<FileStatus>>,
    /// Cache timestamp
    cached_at: Option<SystemTime>,
    /// Cache TTL (time to live)
    ttl: Duration,
    /// Maximum cache age before forced refresh
    max_age: Duration,
}

impl StatusCache {
    /// Create a new status cache
    pub fn new() -> Self {
        Self {
            cached_status: None,
            cached_at: None,
            ttl: Duration::from_secs(30),      // Cache for 30 seconds
            max_age: Duration::from_secs(300), // Force refresh after 5 minutes
        }
    }

    /// Create a cache with custom TTL
    pub fn with_ttl(ttl: Duration) -> Self {
        Self {
            cached_status: None,
            cached_at: None,
            ttl,
            max_age: ttl * 10, // Max age is 10x TTL
        }
    }

    /// Update cache with new status data
    pub fn update(&mut self, status: Vec<FileStatus>) {
        self.cached_status = Some(status);
        self.cached_at = Some(SystemTime::now());
    }

    /// Get cached status if still fresh
    pub fn get_if_fresh(&self) -> Option<Vec<FileStatus>> {
        if let (Some(status), Some(cached_at)) = (&self.cached_status, &self.cached_at) {
            let age = cached_at.elapsed().unwrap_or(Duration::MAX);

            if age < self.ttl {
                return Some(status.clone());
            }
        }

        None
    }

    /// Get cached status regardless of age (for fallback scenarios)
    pub fn get_cached(&self) -> Option<Vec<FileStatus>> {
        self.cached_status.clone()
    }

    /// Check if cache has expired
    pub fn is_expired(&self) -> bool {
        if let Some(cached_at) = &self.cached_at {
            let age = cached_at.elapsed().unwrap_or(Duration::MAX);
            age > self.ttl
        } else {
            true // No cache = expired
        }
    }

    /// Check if cache has exceeded maximum age
    pub fn is_stale(&self) -> bool {
        if let Some(cached_at) = &self.cached_at {
            let age = cached_at.elapsed().unwrap_or(Duration::MAX);
            age > self.max_age
        } else {
            true
        }
    }

    /// Invalidate the cache manually
    pub fn invalidate(&mut self) {
        self.cached_status = None;
        self.cached_at = None;
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            is_cached: self.cached_status.is_some(),
            cached_entries: self.cached_status.as_ref().map(|s| s.len()).unwrap_or(0),
            cached_at: self.cached_at.and_then(|time| {
                time.duration_since(SystemTime::UNIX_EPOCH)
                    .ok()
                    .and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, 0))
            }),
            age_seconds: self
                .cached_at
                .and_then(|time| time.elapsed().ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            is_fresh: !self.is_expired(),
            is_stale: self.is_stale(),
            ttl_seconds: self.ttl.as_secs(),
        }
    }

    /// Set TTL for cache entries
    pub fn set_ttl(&mut self, ttl: Duration) {
        self.ttl = ttl;
        self.max_age = ttl * 10;
    }
}

/// Cache statistics for monitoring
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Whether cache contains data
    pub is_cached: bool,
    /// Number of cached entries
    pub cached_entries: usize,
    /// When cache was last updated
    pub cached_at: Option<DateTime<Utc>>,
    /// Cache age in seconds
    pub age_seconds: u64,
    /// Whether cache is still fresh (within TTL)
    pub is_fresh: bool,
    /// Whether cache is stale (beyond max age)
    pub is_stale: bool,
    /// TTL in seconds
    pub ttl_seconds: u64,
}

impl Default for StatusCache {
    fn default() -> Self {
        Self::new()
    }
}
