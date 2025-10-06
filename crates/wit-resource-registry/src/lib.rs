//! Generic resource registry for managing WIT resource lifecycles
//!
//! This crate provides a reusable solution for managing WebAssembly Interface Types (WIT)
//! resources that need to be accessed in static methods after consumption.
//!
//! ## The Problem
//!
//! WIT resources with static methods like `finish: static func(this: T) -> R` consume the
//! resource handle, and wit-bindgen doesn't provide a way to access the underlying
//! implementation once consumed.
//!
//! ## The Solution
//!
//! This crate provides a generic `ResourceRegistry<T>` that:
//! 1. Stores resource data in `Arc<Mutex<T>>` for shared access
//! 2. Registers the `Arc<Mutex<T>>` in a registry when creating resources
//! 3. Uses the handle ID as the registry key
//! 4. In static methods, retrieves data from registry using the handle ID
//!
//! ## Usage Pattern
//!
//! ```rust,ignore
//! use std::sync::{Arc, Mutex};
//! use wit_resource_registry::ResourceRegistry;
//!
//! // Your resource implementation data
//! pub struct SpanInner {
//!     name: String,
//!     // ... other fields
//! }
//!
//! // Global registry instance
//! static SPAN_REGISTRY: Mutex<Option<ResourceRegistry<SpanInner>>> = Mutex::new(None);
//!
//! // Initialize registry on first use
//! fn ensure_span_registry() {
//!     let mut registry = SPAN_REGISTRY.lock().unwrap();
//!     if registry.is_none() {
//!         *registry = Some(ResourceRegistry::new());
//!     }
//! }
//!
//! // Register resource with handle ID from wit-bindgen
//! pub fn register_span_with_handle(handle: u32, span_data: Arc<Mutex<SpanInner>>) {
//!     ensure_span_registry();
//!     let mut registry = SPAN_REGISTRY.lock().unwrap();
//!     let reg = registry.as_mut().unwrap();
//!     reg.insert(handle, span_data);
//! }
//!
//! // Retrieve resource in static methods
//! pub fn get_span(handle: u32) -> Option<Arc<Mutex<SpanInner>>> {
//!     let registry = SPAN_REGISTRY.lock().unwrap();
//!     registry.as_ref().and_then(|r| r.get(handle))
//! }
//! ```
//!
//! ## Features
//!
//! - **Generic**: Works with any resource type `T`
//! - **Thread-safe**: Uses `Arc<Mutex<T>>` for safe shared access
//! - **Zero-cost abstraction**: No runtime overhead beyond standard library types
//! - **Domain-agnostic**: Reusable across any WIT resource types (telemetry, networking, storage, etc.)
//!
//! ## This pattern is reusable for
//!
//! - Any WIT resources that need lifecycle management beyond wit-bindgen's built-in support

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Generic resource registry for managing WIT resource lifecycles
///
/// This registry maintains a mapping from handle IDs (u32) to resource implementation
/// data wrapped in `Arc<Mutex<T>>`. This allows WIT static methods to access the
/// underlying data even after the handle has been consumed.
///
/// # Type Parameters
///
/// * `T` - The type of resource data being managed (e.g., `SpanInner`, `LogExporterInner`)
///
/// # Thread Safety
///
/// This type is `Send + Sync` when `T: Send`, allowing it to be used in `static` variables
/// wrapped in `Mutex` for global registry instances.
pub struct ResourceRegistry<T> {
    resources: HashMap<u32, Arc<Mutex<T>>>,
    next_id: u32,
}

impl<T> ResourceRegistry<T> {
    /// Create a new empty resource registry
    ///
    /// # Examples
    ///
    /// ```
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let registry: ResourceRegistry<MyResource> = ResourceRegistry::new();
    /// ```
    pub fn new() -> Self {
        Self {
            resources: HashMap::new(),
            next_id: 1,
        }
    }

    /// Register a new resource and return its handle ID
    ///
    /// This method stores the resource data and returns a unique handle ID that can be used
    /// to retrieve or remove the resource later.
    ///
    /// # Arguments
    ///
    /// * `resource` - The resource data wrapped in `Arc<Mutex<T>>`
    ///
    /// # Returns
    ///
    /// A unique handle ID (u32) that identifies this resource in the registry
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let mut registry = ResourceRegistry::new();
    /// let resource = Arc::new(Mutex::new(MyResource {
    ///     name: "test".to_string(),
    /// }));
    ///
    /// let handle = registry.register(resource);
    /// assert_eq!(handle, 1);
    /// ```
    pub fn register(&mut self, resource: Arc<Mutex<T>>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.resources.insert(id, resource);
        id
    }

    /// Insert a resource with a specific handle ID
    ///
    /// This method is useful when the handle ID is provided by wit-bindgen and you need
    /// to associate existing data with that handle.
    ///
    /// # Arguments
    ///
    /// * `id` - The handle ID to use
    /// * `resource` - The resource data wrapped in `Arc<Mutex<T>>`
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let mut registry = ResourceRegistry::new();
    /// let resource = Arc::new(Mutex::new(MyResource {
    ///     name: "test".to_string(),
    /// }));
    ///
    /// // Use a specific handle ID (e.g., from wit-bindgen)
    /// registry.insert(42, resource);
    /// assert!(registry.get(42).is_some());
    /// ```
    pub fn insert(&mut self, id: u32, resource: Arc<Mutex<T>>) {
        self.resources.insert(id, resource);
    }

    /// Get a resource by its handle ID
    ///
    /// Returns a clone of the `Arc<Mutex<T>>` if the resource exists, allowing shared
    /// access to the resource data.
    ///
    /// # Arguments
    ///
    /// * `id` - The handle ID to look up
    ///
    /// # Returns
    ///
    /// `Some(Arc<Mutex<T>>)` if the resource exists, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let mut registry = ResourceRegistry::new();
    /// let resource = Arc::new(Mutex::new(MyResource {
    ///     name: "test".to_string(),
    /// }));
    ///
    /// let handle = registry.register(resource);
    /// let retrieved = registry.get(handle).unwrap();
    /// let data = retrieved.lock().unwrap();
    /// assert_eq!(data.name, "test");
    /// ```
    pub fn get(&self, id: u32) -> Option<Arc<Mutex<T>>> {
        self.resources.get(&id).cloned()
    }

    /// Remove a resource by its handle ID
    ///
    /// This removes the resource from the registry and returns it if it existed.
    /// This is typically called in WIT static methods that consume the resource.
    ///
    /// # Arguments
    ///
    /// * `id` - The handle ID to remove
    ///
    /// # Returns
    ///
    /// `Some(Arc<Mutex<T>>)` if the resource existed, `None` otherwise
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let mut registry = ResourceRegistry::new();
    /// let resource = Arc::new(Mutex::new(MyResource {
    ///     name: "test".to_string(),
    /// }));
    ///
    /// let handle = registry.register(resource);
    /// let removed = registry.remove(handle).unwrap();
    /// assert_eq!(removed.lock().unwrap().name, "test");
    /// assert!(registry.get(handle).is_none());
    /// ```
    pub fn remove(&mut self, id: u32) -> Option<Arc<Mutex<T>>> {
        self.resources.remove(&id)
    }

    /// Get the number of resources currently in the registry
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::{Arc, Mutex};
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let mut registry = ResourceRegistry::new();
    /// assert_eq!(registry.len(), 0);
    ///
    /// let resource = Arc::new(Mutex::new(MyResource {
    ///     name: "test".to_string(),
    /// }));
    /// registry.register(resource);
    /// assert_eq!(registry.len(), 1);
    /// ```
    pub fn len(&self) -> usize {
        self.resources.len()
    }

    /// Check if the registry is empty
    ///
    /// # Examples
    ///
    /// ```
    /// use wit_resource_registry::ResourceRegistry;
    ///
    /// struct MyResource {
    ///     name: String,
    /// }
    ///
    /// let registry: ResourceRegistry<MyResource> = ResourceRegistry::new();
    /// assert!(registry.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        self.resources.is_empty()
    }
}

impl<T> Default for ResourceRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestResource {
        name: String,
        value: i32,
    }

    #[test]
    fn test_new_registry() {
        let registry: ResourceRegistry<TestResource> = ResourceRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_register_resource() {
        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        let handle = registry.register(resource);
        assert_eq!(handle, 1);
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
    }

    #[test]
    fn test_register_multiple_resources() {
        let mut registry = ResourceRegistry::new();

        let handle1 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "first".to_string(),
            value: 1,
        })));
        let handle2 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "second".to_string(),
            value: 2,
        })));

        assert_eq!(handle1, 1);
        assert_eq!(handle2, 2);
        assert_eq!(registry.len(), 2);
    }

    #[test]
    fn test_insert_with_specific_id() {
        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        registry.insert(100, resource);
        assert!(registry.get(100).is_some());
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_get_resource() {
        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        let handle = registry.register(resource);
        let retrieved = registry.get(handle).unwrap();
        let data = retrieved.lock().unwrap();

        assert_eq!(data.name, "test");
        assert_eq!(data.value, 42);
    }

    #[test]
    fn test_get_nonexistent_resource() {
        let registry: ResourceRegistry<TestResource> = ResourceRegistry::new();
        assert!(registry.get(999).is_none());
    }

    #[test]
    fn test_remove_resource() {
        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        let handle = registry.register(resource);
        assert_eq!(registry.len(), 1);

        let removed = registry.remove(handle).unwrap();
        assert_eq!(removed.lock().unwrap().name, "test");
        assert_eq!(registry.len(), 0);
        assert!(registry.get(handle).is_none());
    }

    #[test]
    fn test_remove_nonexistent_resource() {
        let mut registry: ResourceRegistry<TestResource> = ResourceRegistry::new();
        assert!(registry.remove(999).is_none());
    }

    #[test]
    fn test_arc_mutex_sharing() {
        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        let handle = registry.register(resource.clone());

        // Modify through original Arc
        {
            let mut data = resource.lock().unwrap();
            data.value = 100;
        }

        // Retrieve from registry and verify change
        let retrieved = registry.get(handle).unwrap();
        let data = retrieved.lock().unwrap();
        assert_eq!(data.value, 100);
    }

    #[test]
    fn test_default_implementation() {
        let registry: ResourceRegistry<TestResource> = Default::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_overwrite_existing_id() {
        let mut registry = ResourceRegistry::new();

        let resource1 = Arc::new(Mutex::new(TestResource {
            name: "first".to_string(),
            value: 1,
        }));
        let resource2 = Arc::new(Mutex::new(TestResource {
            name: "second".to_string(),
            value: 2,
        }));

        registry.insert(42, resource1);
        registry.insert(42, resource2);

        let retrieved = registry.get(42).unwrap();
        let data = retrieved.lock().unwrap();
        assert_eq!(data.name, "second");
        assert_eq!(data.value, 2);
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_concurrent_access_simulation() {
        use std::sync::Arc;

        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "concurrent".to_string(),
            value: 0,
        }));

        let handle = registry.register(resource);

        // Simulate multiple "threads" accessing through registry
        for i in 1..=10 {
            let retrieved = registry.get(handle).unwrap();
            let mut data = retrieved.lock().unwrap();
            data.value += i;
        }

        let final_resource = registry.get(handle).unwrap();
        let data = final_resource.lock().unwrap();
        // Sum of 1..=10 is 55
        assert_eq!(data.value, 55);
    }

    #[test]
    fn test_handle_id_sequence() {
        let mut registry = ResourceRegistry::new();

        let handle1 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "first".to_string(),
            value: 1,
        })));
        let handle2 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "second".to_string(),
            value: 2,
        })));
        let handle3 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "third".to_string(),
            value: 3,
        })));

        assert_eq!(handle1, 1);
        assert_eq!(handle2, 2);
        assert_eq!(handle3, 3);

        // Remove middle one
        registry.remove(handle2);

        // Next registration should be 4 (sequence continues)
        let handle4 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "fourth".to_string(),
            value: 4,
        })));
        assert_eq!(handle4, 4);
    }

    #[test]
    fn test_len_and_is_empty_after_operations() {
        let mut registry = ResourceRegistry::new();
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());

        let handle1 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "first".to_string(),
            value: 1,
        })));
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let handle2 = registry.register(Arc::new(Mutex::new(TestResource {
            name: "second".to_string(),
            value: 2,
        })));
        assert_eq!(registry.len(), 2);

        registry.remove(handle1);
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        registry.remove(handle2);
        assert_eq!(registry.len(), 0);
        assert!(registry.is_empty());
    }

    #[test]
    fn test_insert_vs_register_behavior() {
        let mut registry = ResourceRegistry::new();

        // Using register - automatic ID assignment
        let auto_handle = registry.register(Arc::new(Mutex::new(TestResource {
            name: "auto".to_string(),
            value: 1,
        })));
        assert_eq!(auto_handle, 1);

        // Using insert - manual ID assignment
        registry.insert(100, Arc::new(Mutex::new(TestResource {
            name: "manual".to_string(),
            value: 100,
        })));

        // Next register should be 2 (not affected by manual insert)
        let next_auto = registry.register(Arc::new(Mutex::new(TestResource {
            name: "auto2".to_string(),
            value: 2,
        })));
        assert_eq!(next_auto, 2);

        assert!(registry.get(1).is_some());
        assert!(registry.get(2).is_some());
        assert!(registry.get(100).is_some());
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn test_get_returns_cloned_arc() {
        let mut registry = ResourceRegistry::new();
        let original_arc = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        let handle = registry.register(original_arc.clone());

        // Get from registry
        let retrieved1 = registry.get(handle).unwrap();
        let retrieved2 = registry.get(handle).unwrap();

        // Both should be valid Arc clones
        assert_eq!(Arc::strong_count(&retrieved1), 4); // original + registry + retrieved1 + retrieved2
        assert_eq!(Arc::strong_count(&retrieved2), 4);

        // Modify through one Arc
        {
            let mut data = retrieved1.lock().unwrap();
            data.value = 100;
        }

        // Visible through other Arc
        {
            let data = retrieved2.lock().unwrap();
            assert_eq!(data.value, 100);
        }
    }

    #[test]
    fn test_remove_returns_owned_arc() {
        let mut registry = ResourceRegistry::new();
        let resource = Arc::new(Mutex::new(TestResource {
            name: "test".to_string(),
            value: 42,
        }));

        let handle = registry.register(resource.clone());
        assert_eq!(Arc::strong_count(&resource), 2); // original + registry

        let removed = registry.remove(handle).unwrap();
        assert_eq!(Arc::strong_count(&resource), 2); // original + removed (no longer in registry)

        // Verify it's the same resource
        {
            let data = removed.lock().unwrap();
            assert_eq!(data.name, "test");
            assert_eq!(data.value, 42);
        }

        // No longer in registry
        assert!(registry.get(handle).is_none());
    }

    #[test]
    fn test_zero_id_is_valid() {
        let mut registry = ResourceRegistry::new();

        // Manually insert at ID 0
        registry.insert(0, Arc::new(Mutex::new(TestResource {
            name: "zero".to_string(),
            value: 0,
        })));

        assert!(registry.get(0).is_some());
        let data = registry.get(0).unwrap();
        assert_eq!(data.lock().unwrap().name, "zero");
    }

    #[test]
    fn test_max_u32_id() {
        let mut registry = ResourceRegistry::new();

        // Use maximum u32 value
        registry.insert(u32::MAX, Arc::new(Mutex::new(TestResource {
            name: "max".to_string(),
            value: i32::MAX,
        })));

        assert!(registry.get(u32::MAX).is_some());
        assert_eq!(registry.len(), 1);

        let removed = registry.remove(u32::MAX);
        assert!(removed.is_some());
        assert_eq!(registry.len(), 0);
    }
}
