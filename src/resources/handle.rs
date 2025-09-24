//! # Resource Handle System
//!
//! Smart pointer implementation for GPU resources with automatic reference counting,
//! lifecycle management, and integration with the resource pool system.

use super::{ManagedResource, ResourceManager};
use crate::error::{HaggisError, HaggisResult};
use std::sync::{Arc, Weak, atomic::{AtomicUsize, Ordering}};
use std::ops::{Deref, DerefMut};
use std::fmt;

/// A unique identifier for resources in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResourceId(pub u64);

impl ResourceId {
    /// Generate a new unique resource ID
    pub fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst) as u64)
    }
}

/// Smart pointer for managed GPU resources with automatic reference counting
///
/// ResourceHandle provides automatic cleanup, reference counting, and integration
/// with the resource pool system. When the last handle to a resource is dropped,
/// the resource is either returned to the pool (if poolable) or destroyed.
///
/// # Type Safety
///
/// ResourceHandle is strongly typed and can only hold resources of type T.
/// This prevents resource type confusion and enables compile-time validation.
///
/// # Examples
///
/// ```no_run
/// use haggis::resources::{ResourceHandle, ResourceManager};
/// use wgpu::Buffer;
///
/// let manager = ResourceManager::new();
/// let buffer_handle: ResourceHandle<Buffer> = manager.create_buffer(
///     &device,
///     &descriptor
/// ).await?;
///
/// // Automatically derefs to &Buffer
/// let size = buffer_handle.size();
///
/// // Handle automatically cleans up when dropped
/// ```
pub struct ResourceHandle<T: ManagedResource> {
    /// Unique identifier for this resource
    id: ResourceId,

    /// The actual resource data
    resource: Arc<T>,

    /// Weak reference to the resource manager for cleanup
    manager: Weak<ResourceManager>,

    /// Reference count for debugging
    ref_count: Arc<AtomicUsize>,
}

impl<T: ManagedResource> ResourceHandle<T> {
    /// Create a new resource handle
    pub(crate) fn new(
        resource: T,
        manager: Weak<ResourceManager>,
    ) -> Self {
        let id = ResourceId::new();
        let resource = Arc::new(resource);
        let ref_count = Arc::new(AtomicUsize::new(1));

        Self {
            id,
            resource,
            manager,
            ref_count,
        }
    }

    /// Get the unique ID of this resource
    pub fn id(&self) -> ResourceId {
        self.id
    }

    /// Get the current reference count
    pub fn ref_count(&self) -> usize {
        Arc::strong_count(&self.resource)
    }

    /// Check if this is the only reference to the resource
    pub fn is_unique(&self) -> bool {
        Arc::strong_count(&self.resource) == 1
    }

    /// Get the estimated memory usage of this resource
    pub fn memory_usage(&self) -> u64 {
        self.resource.estimated_size()
    }

    /// Get the resource type name
    pub fn type_name(&self) -> &'static str {
        T::resource_type_name()
    }

    /// Create a weak reference to this resource
    pub fn downgrade(&self) -> WeakResourceHandle<T> {
        WeakResourceHandle {
            id: self.id,
            resource: Arc::downgrade(&self.resource),
            manager: self.manager.clone(),
        }
    }

    /// Try to get mutable access to the resource
    /// Returns None if there are other references to the resource
    pub fn try_get_mut(&mut self) -> Option<&mut T> {
        Arc::get_mut(&mut self.resource)
    }

    /// Force unique access to the resource by cloning if necessary
    /// This should be used sparingly as it can be expensive
    pub fn make_mut(&mut self) -> HaggisResult<&mut T>
    where
        T: Clone,
    {
        if Arc::strong_count(&self.resource) > 1 {
            // Clone the resource to get unique access
            let cloned = (*self.resource).clone();
            self.resource = Arc::new(cloned);
            self.id = ResourceId::new(); // New resource gets new ID
        }
        Ok(Arc::get_mut(&mut self.resource).unwrap())
    }

    /// Check if the resource can be pooled
    pub fn is_poolable(&self) -> bool {
        self.resource.is_poolable()
    }

    /// Get a raw pointer to the resource (unsafe)
    /// This should only be used for FFI or very specific performance scenarios
    pub unsafe fn as_ptr(&self) -> *const T {
        Arc::as_ptr(&self.resource)
    }
}

impl<T: ManagedResource> Clone for ResourceHandle<T> {
    fn clone(&self) -> Self {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
        Self {
            id: self.id,
            resource: Arc::clone(&self.resource),
            manager: self.manager.clone(),
            ref_count: Arc::clone(&self.ref_count),
        }
    }
}

impl<T: ManagedResource> Drop for ResourceHandle<T> {
    fn drop(&mut self) {
        let old_count = self.ref_count.fetch_sub(1, Ordering::SeqCst);

        // If this was the last reference, handle cleanup
        if old_count == 1 {
            if let Some(manager) = self.manager.upgrade() {
                // Try to return to pool if possible, otherwise it will be dropped
                if self.resource.is_poolable() && Arc::strong_count(&self.resource) == 1 {
                    // Note: In a real implementation, we'd need access to the pool
                    // This is a simplified version for demonstration
                    let _ = manager.try_return_to_pool(self.id, Arc::clone(&self.resource));
                }
            }
        }
    }
}

impl<T: ManagedResource> Deref for ResourceHandle<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.resource
    }
}

impl<T: ManagedResource> AsRef<T> for ResourceHandle<T> {
    fn as_ref(&self) -> &T {
        &self.resource
    }
}

impl<T: ManagedResource> fmt::Debug for ResourceHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResourceHandle")
            .field("id", &self.id)
            .field("type", &self.type_name())
            .field("ref_count", &self.ref_count())
            .field("memory_usage", &self.memory_usage())
            .finish()
    }
}

impl<T: ManagedResource> fmt::Display for ResourceHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}(id={:?}, refs={}, size={})",
            self.type_name(),
            self.id,
            self.ref_count(),
            self.memory_usage()
        )
    }
}

/// Weak reference to a resource that doesn't keep it alive
///
/// WeakResourceHandle allows you to hold a reference to a resource without
/// preventing its cleanup. Useful for observers, caches, or temporary references.
pub struct WeakResourceHandle<T: ManagedResource> {
    id: ResourceId,
    resource: Weak<T>,
    manager: Weak<ResourceManager>,
}

impl<T: ManagedResource> WeakResourceHandle<T> {
    /// Get the resource ID
    pub fn id(&self) -> ResourceId {
        self.id
    }

    /// Try to upgrade to a strong reference
    /// Returns None if the resource has been deallocated
    pub fn upgrade(&self) -> Option<ResourceHandle<T>> {
        self.resource.upgrade().map(|resource| {
            ResourceHandle {
                id: self.id,
                resource,
                manager: self.manager.clone(),
                ref_count: Arc::new(AtomicUsize::new(1)),
            }
        })
    }

    /// Check if the resource is still alive
    pub fn is_alive(&self) -> bool {
        self.resource.strong_count() > 0
    }

    /// Get the current strong reference count
    pub fn strong_count(&self) -> usize {
        self.resource.strong_count()
    }
}

impl<T: ManagedResource> Clone for WeakResourceHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            resource: self.resource.clone(),
            manager: self.manager.clone(),
        }
    }
}

impl<T: ManagedResource> fmt::Debug for WeakResourceHandle<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WeakResourceHandle")
            .field("id", &self.id)
            .field("type", &T::resource_type_name())
            .field("strong_count", &self.strong_count())
            .field("is_alive", &self.is_alive())
            .finish()
    }
}

/// Convenience trait for converting resources into handles
pub trait IntoHandle<T: ManagedResource> {
    fn into_handle(self, manager: Weak<ResourceManager>) -> ResourceHandle<T>;
}

impl<T: ManagedResource> IntoHandle<T> for T {
    fn into_handle(self, manager: Weak<ResourceManager>) -> ResourceHandle<T> {
        ResourceHandle::new(self, manager)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    // Mock resource for testing
    #[derive(Debug, Clone)]
    struct MockResource {
        size: u64,
        poolable: bool,
    }

    impl ManagedResource for MockResource {
        fn resource_type_name() -> &'static str {
            "MockResource"
        }

        fn estimated_size(&self) -> u64 {
            self.size
        }

        fn is_poolable(&self) -> bool {
            self.poolable
        }
    }

    #[test]
    fn test_resource_handle_creation() {
        let resource = MockResource { size: 1024, poolable: true };
        let manager = Weak::new();
        let handle = ResourceHandle::new(resource, manager);

        assert_eq!(handle.memory_usage(), 1024);
        assert_eq!(handle.type_name(), "MockResource");
        assert_eq!(handle.ref_count(), 1);
        assert!(handle.is_unique());
        assert!(handle.is_poolable());
    }

    #[test]
    fn test_resource_handle_clone() {
        let resource = MockResource { size: 512, poolable: false };
        let manager = Weak::new();
        let handle1 = ResourceHandle::new(resource, manager);
        let handle2 = handle1.clone();

        assert_eq!(handle1.id(), handle2.id());
        assert_eq!(handle1.ref_count(), 2);
        assert_eq!(handle2.ref_count(), 2);
        assert!(!handle1.is_unique());
        assert!(!handle2.is_unique());
    }

    #[test]
    fn test_weak_handle() {
        let resource = MockResource { size: 256, poolable: true };
        let manager = Weak::new();
        let handle = ResourceHandle::new(resource, manager);
        let weak = handle.downgrade();

        assert_eq!(weak.id(), handle.id());
        assert!(weak.is_alive());
        assert_eq!(weak.strong_count(), 1);

        drop(handle);
        assert!(!weak.is_alive());
        assert_eq!(weak.strong_count(), 0);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn test_resource_id_uniqueness() {
        let id1 = ResourceId::new();
        let id2 = ResourceId::new();
        let id3 = ResourceId::new();

        assert_ne!(id1, id2);
        assert_ne!(id2, id3);
        assert_ne!(id1, id3);
    }
}