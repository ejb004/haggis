//! # Resource Pool System
//!
//! Object pooling implementation for frequently allocated and deallocated GPU resources.
//! Reduces allocation overhead and memory fragmentation by reusing resources.

use super::{ManagedResource, ResourceConfig};
use super::handle::ResourceId;
use crate::error::{HaggisError, HaggisResult};
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, atomic::{AtomicU64, Ordering}};
use std::time::{SystemTime, UNIX_EPOCH};
use std::any::{TypeId, Any};

/// Wrapper for pooled resources that tracks usage statistics
pub struct PooledResource<T: ManagedResource> {
    /// The actual resource
    pub resource: T,

    /// When this resource was created
    pub created_at: u64,

    /// When this resource was last used
    pub last_used: u64,

    /// How many times this resource has been reused
    pub usage_count: u64,

    /// Original size when allocated (for validation)
    pub original_size: u64,
}

impl<T: ManagedResource> PooledResource<T> {
    fn new(resource: T) -> Self {
        let now = current_timestamp();
        let original_size = resource.estimated_size();

        Self {
            resource,
            created_at: now,
            last_used: now,
            usage_count: 0,
            original_size,
        }
    }

    fn mark_used(&mut self) {
        self.last_used = current_timestamp();
        self.usage_count += 1;
    }

    fn age_seconds(&self) -> f64 {
        (current_timestamp() - self.created_at) as f64 / 1000.0
    }

    fn idle_seconds(&self) -> f64 {
        (current_timestamp() - self.last_used) as f64 / 1000.0
    }
}

/// A pool for a specific resource type
struct TypedPool<T: ManagedResource> {
    /// Available resources ready for reuse
    available: VecDeque<PooledResource<T>>,

    /// Maximum number of resources to keep in pool
    max_size: usize,

    /// Total hits (successful reuse)
    hits: AtomicU64,

    /// Total misses (new allocations)
    misses: AtomicU64,

    /// Resource creation function
    factory: Option<Box<dyn Fn() -> HaggisResult<T> + Send + Sync>>,
}

impl<T: ManagedResource> TypedPool<T> {
    fn new(max_size: usize) -> Self {
        Self {
            available: VecDeque::new(),
            max_size,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            factory: None,
        }
    }

    fn with_factory<F>(max_size: usize, factory: F) -> Self
    where
        F: Fn() -> HaggisResult<T> + Send + Sync + 'static,
    {
        Self {
            available: VecDeque::new(),
            max_size,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
            factory: Some(Box::new(factory)),
        }
    }

    /// Try to get a resource from the pool
    fn try_acquire(&mut self) -> Option<PooledResource<T>> {
        if let Some(mut resource) = self.available.pop_front() {
            resource.mark_used();
            self.hits.fetch_add(1, Ordering::Relaxed);
            Some(resource)
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
            None
        }
    }

    /// Return a resource to the pool
    fn return_resource(&mut self, mut resource: PooledResource<T>) -> HaggisResult<()> {
        // Validate the resource can be safely reused
        if resource.resource.estimated_size() != resource.original_size {
            return Err(HaggisError::validation(
                "Resource size changed, cannot be safely pooled"
            ));
        }

        // Reset the resource to a clean state
        resource.resource.reset()?;
        resource.mark_used();

        // Add to pool if there's space
        if self.available.len() < self.max_size {
            self.available.push_back(resource);
        }
        // If pool is full, resource will be dropped

        Ok(())
    }

    /// Create a new resource using the factory
    fn create_new(&mut self) -> HaggisResult<PooledResource<T>> {
        if let Some(factory) = &self.factory {
            let resource = factory()?;
            self.misses.fetch_add(1, Ordering::Relaxed);
            Ok(PooledResource::new(resource))
        } else {
            Err(HaggisError::resource(
                "No factory function configured for this resource type",
                "ResourcePool"
            ))
        }
    }

    /// Clean up old resources
    fn cleanup_old(&mut self, max_idle_seconds: f64) {
        let now = current_timestamp();
        self.available.retain(|resource| {
            let idle_time = (now - resource.last_used) as f64 / 1000.0;
            idle_time < max_idle_seconds
        });
    }

    fn hits(&self) -> u64 {
        self.hits.load(Ordering::Relaxed)
    }

    fn misses(&self) -> u64 {
        self.misses.load(Ordering::Relaxed)
    }

    fn hit_rate(&self) -> f32 {
        let hits = self.hits();
        let misses = self.misses();
        if hits + misses == 0 {
            0.0
        } else {
            (hits as f32) / ((hits + misses) as f32) * 100.0
        }
    }
}

/// Resource pool manager for all resource types
pub struct ResourcePool {
    /// Type-erased pools indexed by TypeId
    pools: Mutex<HashMap<TypeId, Box<dyn Any + Send + Sync>>>,

    /// Configuration
    config: ResourceConfig,

    /// Global statistics
    total_hits: AtomicU64,
    total_misses: AtomicU64,
    cleanup_count: AtomicU64,
}

impl ResourcePool {
    /// Create a new resource pool with default configuration
    pub fn new() -> Self {
        Self::with_config(ResourceConfig::default())
    }

    /// Create a new resource pool with custom configuration
    pub fn with_config(config: ResourceConfig) -> Self {
        Self {
            pools: Mutex::new(HashMap::new()),
            config,
            total_hits: AtomicU64::new(0),
            total_misses: AtomicU64::new(0),
            cleanup_count: AtomicU64::new(0),
        }
    }

    /// Register a factory function for a resource type
    pub fn register_factory<T, F>(&self, factory: F) -> HaggisResult<()>
    where
        T: ManagedResource + 'static,
        F: Fn() -> HaggisResult<T> + Send + Sync + 'static,
    {
        if !self.config.enable_pooling {
            return Ok(()); // Pooling disabled
        }

        let mut pools = self.pools.lock().map_err(|_| {
            HaggisError::resource("Failed to acquire pool lock", "ResourcePool")
        })?;

        let type_id = TypeId::of::<T>();
        let pool = TypedPool::with_factory(self.config.max_pool_size, factory);
        pools.insert(type_id, Box::new(Mutex::new(pool)));

        Ok(())
    }

    /// Try to acquire a resource from the pool
    pub fn try_acquire<T>(&self) -> Option<PooledResource<T>>
    where
        T: ManagedResource + 'static,
    {
        if !self.config.enable_pooling {
            return None;
        }

        let pools = self.pools.lock().ok()?;
        let type_id = TypeId::of::<T>();

        if let Some(pool_any) = pools.get(&type_id) {
            if let Some(pool) = pool_any.downcast_ref::<Mutex<TypedPool<T>>>() {
                if let Ok(mut typed_pool) = pool.lock() {
                    if let Some(resource) = typed_pool.try_acquire() {
                        self.total_hits.fetch_add(1, Ordering::Relaxed);
                        return Some(resource);
                    }
                }
            }
        }

        self.total_misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    /// Create a new resource using the registered factory
    pub fn create_or_acquire<T>(&self) -> HaggisResult<PooledResource<T>>
    where
        T: ManagedResource + 'static,
    {
        // First try to acquire from pool
        if let Some(resource) = self.try_acquire::<T>() {
            return Ok(resource);
        }

        // If not available in pool, create new using factory
        let pools = self.pools.lock().map_err(|_| {
            HaggisError::resource("Failed to acquire pool lock", "ResourcePool")
        })?;

        let type_id = TypeId::of::<T>();
        if let Some(pool_any) = pools.get(&type_id) {
            if let Some(pool) = pool_any.downcast_ref::<Mutex<TypedPool<T>>>() {
                if let Ok(mut typed_pool) = pool.lock() {
                    return typed_pool.create_new();
                }
            }
        }

        Err(HaggisError::resource(
            format!("No factory registered for resource type: {}", T::resource_type_name()),
            "ResourcePool"
        ))
    }

    /// Return a resource to the pool
    pub fn return_resource<T>(&self, resource: PooledResource<T>) -> HaggisResult<()>
    where
        T: ManagedResource + 'static,
    {
        if !self.config.enable_pooling || !resource.resource.is_poolable() {
            return Ok(()); // Just drop the resource
        }

        let pools = self.pools.lock().map_err(|_| {
            HaggisError::resource("Failed to acquire pool lock", "ResourcePool")
        })?;

        let type_id = TypeId::of::<T>();
        if let Some(pool_any) = pools.get(&type_id) {
            if let Some(pool) = pool_any.downcast_ref::<Mutex<TypedPool<T>>>() {
                if let Ok(mut typed_pool) = pool.lock() {
                    return typed_pool.return_resource(resource);
                }
            }
        }

        // No pool found, just drop the resource
        Ok(())
    }

    /// Perform cleanup of old resources
    pub fn cleanup(&self) -> HaggisResult<usize> {
        let mut cleaned = 0;
        let max_idle = self.config.cleanup_interval as f64;

        let pools = self.pools.lock().map_err(|_| {
            HaggisError::resource("Failed to acquire pool lock", "ResourcePool")
        })?;

        for (_, pool_any) in pools.iter() {
            // This is a bit tricky due to type erasure
            // In a real implementation, we'd need a trait for cleanup
            cleaned += 1; // Placeholder
        }

        self.cleanup_count.fetch_add(1, Ordering::Relaxed);
        Ok(cleaned)
    }

    /// Get global pool statistics
    pub fn get_statistics(&self) -> PoolStatistics {
        PoolStatistics {
            total_hits: self.total_hits.load(Ordering::Relaxed),
            total_misses: self.total_misses.load(Ordering::Relaxed),
            cleanup_count: self.cleanup_count.load(Ordering::Relaxed),
            enabled: self.config.enable_pooling,
        }
    }

    /// Get statistics for a specific resource type
    pub fn get_type_statistics<T>(&self) -> Option<TypeStatistics>
    where
        T: ManagedResource + 'static,
    {
        if !self.config.enable_pooling {
            return None;
        }

        let pools = self.pools.lock().ok()?;
        let type_id = TypeId::of::<T>();

        if let Some(pool_any) = pools.get(&type_id) {
            if let Some(pool) = pool_any.downcast_ref::<Mutex<TypedPool<T>>>() {
                if let Ok(typed_pool) = pool.lock() {
                    return Some(TypeStatistics {
                        type_name: T::resource_type_name(),
                        available_count: typed_pool.available.len(),
                        hits: typed_pool.hits(),
                        misses: typed_pool.misses(),
                        hit_rate: typed_pool.hit_rate(),
                    });
                }
            }
        }

        None
    }

    /// Check if pooling is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enable_pooling
    }
}

impl Default for ResourcePool {
    fn default() -> Self {
        Self::new()
    }
}

/// Global pool statistics
#[derive(Debug, Clone)]
pub struct PoolStatistics {
    pub total_hits: u64,
    pub total_misses: u64,
    pub cleanup_count: u64,
    pub enabled: bool,
}

impl PoolStatistics {
    pub fn hit_rate(&self) -> f32 {
        if self.total_hits + self.total_misses == 0 {
            0.0
        } else {
            (self.total_hits as f32) / ((self.total_hits + self.total_misses) as f32) * 100.0
        }
    }
}

/// Statistics for a specific resource type
#[derive(Debug, Clone)]
pub struct TypeStatistics {
    pub type_name: &'static str,
    pub available_count: usize,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f32,
}

/// Get current timestamp in milliseconds
fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize};

    // Mock resource for testing
    #[derive(Debug, Clone)]
    struct MockResource {
        id: usize,
        size: u64,
        reset_called: Arc<AtomicBool>,
    }

    impl ManagedResource for MockResource {
        fn resource_type_name() -> &'static str {
            "MockResource"
        }

        fn estimated_size(&self) -> u64 {
            self.size
        }

        fn is_poolable(&self) -> bool {
            true
        }

        fn reset(&mut self) -> HaggisResult<()> {
            self.reset_called.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn test_pooled_resource() {
        let resource = MockResource {
            id: 1,
            size: 1024,
            reset_called: Arc::new(AtomicBool::new(false)),
        };

        let mut pooled = PooledResource::new(resource);
        assert_eq!(pooled.usage_count, 0);
        assert_eq!(pooled.original_size, 1024);

        pooled.mark_used();
        assert_eq!(pooled.usage_count, 1);
        assert!(pooled.idle_seconds() < 0.1); // Just used
    }

    #[test]
    fn test_typed_pool() {
        let mut pool = TypedPool::<MockResource>::new(5);

        // Pool should be empty initially
        assert!(pool.try_acquire().is_none());
        assert_eq!(pool.misses(), 1);

        // Return a resource to the pool
        let resource = MockResource {
            id: 1,
            size: 512,
            reset_called: Arc::new(AtomicBool::new(false)),
        };
        let pooled = PooledResource::new(resource);
        let reset_flag = pooled.resource.reset_called.clone();

        assert!(pool.return_resource(pooled).is_ok());
        assert!(reset_flag.load(Ordering::Relaxed)); // Reset should have been called

        // Now we should be able to acquire it
        let acquired = pool.try_acquire().unwrap();
        assert_eq!(acquired.resource.id, 1);
        assert_eq!(pool.hits(), 1);
    }

    #[test]
    fn test_resource_pool() {
        let pool = ResourcePool::new();
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        // Register a factory
        pool.register_factory::<MockResource, _>(|| {
            let id = COUNTER.fetch_add(1, Ordering::Relaxed);
            Ok(MockResource {
                id,
                size: 1024,
                reset_called: Arc::new(AtomicBool::new(false)),
            })
        }).unwrap();

        // Create a new resource (should use factory)
        let resource1 = pool.create_or_acquire::<MockResource>().unwrap();
        assert_eq!(resource1.resource.id, 0);

        // Return it to the pool
        pool.return_resource(resource1).unwrap();

        // Acquire again (should come from pool)
        let resource2 = pool.create_or_acquire::<MockResource>().unwrap();
        assert_eq!(resource2.resource.id, 0); // Same resource
        assert!(resource2.usage_count > 0); // Should have been marked as used

        let stats = pool.get_statistics();
        assert!(stats.total_hits > 0);
    }

    #[test]
    fn test_pool_statistics() {
        let stats = PoolStatistics {
            total_hits: 8,
            total_misses: 2,
            cleanup_count: 1,
            enabled: true,
        };

        assert_eq!(stats.hit_rate(), 80.0);
    }
}