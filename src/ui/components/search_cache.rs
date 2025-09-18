//! Search result caching system for improved performance

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use super::search::{SearchResult, SearchScope, FilterCriteria};

/// Cache key for search results
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub query: String,
    pub scope: SearchScope,
    pub filter_hash: u64,
}

impl CacheKey {
    pub fn new(query: &str, scope: SearchScope, filter: &FilterCriteria) -> Self {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();

        // Hash the filter criteria
        if let Some(ref author) = filter.author {
            author.hash(&mut hasher);
        }
        if let Some(ref date_from) = filter.date_from {
            date_from.hash(&mut hasher);
        }
        if let Some(ref date_to) = filter.date_to {
            date_to.hash(&mut hasher);
        }
        if let Some(ref file_ext) = filter.file_extension {
            file_ext.hash(&mut hasher);
        }
        if let Some(ref file_path) = filter.file_path {
            file_path.hash(&mut hasher);
        }
        if let Some(ref content_type) = filter.content_type {
            content_type.hash(&mut hasher);
        }

        Self {
            query: query.to_string(),
            scope,
            filter_hash: hasher.finish(),
        }
    }
}

/// Cached search result entry
#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub results: Vec<SearchResult>,
    pub timestamp: Instant,
    pub hit_count: usize,
}

impl CacheEntry {
    pub fn new(results: Vec<SearchResult>) -> Self {
        Self {
            results,
            timestamp: Instant::now(),
            hit_count: 0,
        }
    }

    pub fn is_expired(&self, ttl: Duration) -> bool {
        self.timestamp.elapsed() > ttl
    }

    pub fn record_hit(&mut self) {
        self.hit_count += 1;
    }
}

/// Search result cache with LRU eviction and TTL
pub struct SearchCache {
    cache: HashMap<CacheKey, CacheEntry>,
    max_entries: usize,
    ttl: Duration,
    access_order: Vec<CacheKey>,
}

impl SearchCache {
    /// Create a new search cache
    pub fn new(max_entries: usize, ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_entries,
            ttl,
            access_order: Vec::new(),
        }
    }

    /// Get cached search results
    pub fn get(&mut self, key: &CacheKey) -> Option<Vec<SearchResult>> {
        // Check if entry exists and is not expired
        let is_expired = if let Some(entry) = self.cache.get(key) {
            entry.is_expired(self.ttl)
        } else {
            return None;
        };

        if is_expired {
            // Remove expired entry
            self.cache.remove(key);
            self.access_order.retain(|k| k != key);
            return None;
        }

        // Entry exists and is not expired
        if let Some(entry) = self.cache.get_mut(key) {
            entry.record_hit();

            // Update access order for LRU
            if let Some(pos) = self.access_order.iter().position(|k| k == key) {
                let key_copy = self.access_order.remove(pos);
                self.access_order.push(key_copy);
            }

            return Some(entry.results.clone());
        }

        None
    }

    /// Insert search results into cache
    pub fn insert(&mut self, key: CacheKey, results: Vec<SearchResult>) {
        // Remove oldest entries if cache is full
        while self.cache.len() >= self.max_entries {
            if let Some(oldest_key) = self.access_order.first().cloned() {
                self.cache.remove(&oldest_key);
                self.access_order.remove(0);
            } else {
                break;
            }
        }

        // Insert new entry
        let entry = CacheEntry::new(results);
        self.cache.insert(key.clone(), entry);
        self.access_order.push(key);
    }

    /// Clear expired entries
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        let expired_keys: Vec<CacheKey> = self.cache
            .iter()
            .filter(|(_, entry)| entry.is_expired(self.ttl))
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.cache.remove(&key);
            self.access_order.retain(|k| k != &key);
        }
    }

    /// Clear all cache entries
    pub fn clear(&mut self) {
        self.cache.clear();
        self.access_order.clear();
    }

    /// Get cache statistics
    pub fn stats(&self) -> CacheStats {
        let total_hits: usize = self.cache.values().map(|entry| entry.hit_count).sum();
        let total_entries = self.cache.len();
        let expired_count = self.cache.values()
            .filter(|entry| entry.is_expired(self.ttl))
            .count();

        CacheStats {
            total_entries,
            expired_entries: expired_count,
            total_hits,
            max_entries: self.max_entries,
            ttl: self.ttl,
        }
    }

    /// Get memory usage estimate in bytes
    pub fn estimated_memory_usage(&self) -> usize {
        let mut total = 0;

        for (key, entry) in &self.cache {
            // Size of key
            total += std::mem::size_of::<CacheKey>();
            total += key.query.len();

            // Size of entry
            total += std::mem::size_of::<CacheEntry>();

            // Size of results
            for result in &entry.results {
                total += std::mem::size_of::<SearchResult>();
                total += result.title.len();
                total += result.content.len();
                if let Some(ref path) = result.file_path {
                    total += path.len();
                }
                if let Some(ref hash) = result.commit_hash {
                    total += hash.len();
                }
                if let Some(ref branch) = result.branch_name {
                    total += branch.len();
                }
            }
        }

        // Add access order vector
        total += self.access_order.len() * std::mem::size_of::<CacheKey>();

        total
    }

    /// Preload common searches
    pub fn preload_common_searches(&mut self, common_queries: &[(String, SearchScope)]) {
        // This could be implemented to preload frequently used searches
        // For now, it's a placeholder for future optimization
        for (query, scope) in common_queries {
            let key = CacheKey::new(query, scope.clone(), &FilterCriteria::default());
            // Would trigger background search and cache results
            println!("Would preload search: {} in scope: {:?}", query, scope);
        }
    }

    /// Warm up cache with recent Git activity
    pub fn warm_up_with_git_data(&mut self) {
        // This could be implemented to automatically cache recent commits,
        // modified files, etc. based on Git activity
        // For now, it's a placeholder for future optimization
        println!("Would warm up cache with recent Git data");
    }
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub expired_entries: usize,
    pub total_hits: usize,
    pub max_entries: usize,
    pub ttl: Duration,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_entries == 0 {
            0.0
        } else {
            self.total_hits as f64 / self.total_entries as f64
        }
    }

    pub fn utilization(&self) -> f64 {
        self.total_entries as f64 / self.max_entries as f64
    }
}

impl Default for SearchCache {
    fn default() -> Self {
        // Default: 100 entries, 5 minute TTL
        Self::new(100, Duration::from_secs(300))
    }
}