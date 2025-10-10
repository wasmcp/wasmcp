//! Memory tracking utilities for testing bounded memory usage
//!
//! This module provides a global allocator wrapper that tracks peak memory usage
//! during streaming operations. It's used to verify that memory usage remains
//! bounded regardless of content size.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

/// Global allocator that tracks allocations and peak memory usage
pub struct TrackingAllocator;

/// Current total allocated bytes
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);

/// Peak allocated bytes observed
static PEAK: AtomicUsize = AtomicUsize::new(0);

/// Number of allocations performed
static ALLOC_COUNT: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size();
        let current = ALLOCATED.fetch_add(size, Ordering::Relaxed) + size;
        ALLOC_COUNT.fetch_add(1, Ordering::Relaxed);

        // Update peak using compare-exchange loop
        let mut peak = PEAK.load(Ordering::Relaxed);
        while current > peak {
            match PEAK.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(p) => peak = p,
            }
        }

        System.alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        ALLOCATED.fetch_sub(layout.size(), Ordering::Relaxed);
        System.dealloc(ptr, layout)
    }
}

/// Memory usage statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    /// Current allocated bytes
    pub current: usize,
    /// Peak allocated bytes
    pub peak: usize,
    /// Number of allocations
    pub alloc_count: usize,
}

/// Get current memory usage statistics
pub fn get_stats() -> MemoryStats {
    MemoryStats {
        current: ALLOCATED.load(Ordering::Relaxed),
        peak: PEAK.load(Ordering::Relaxed),
        alloc_count: ALLOC_COUNT.load(Ordering::Relaxed),
    }
}

/// Reset peak tracking (keeps current allocation count)
pub fn reset_peak() {
    let current = ALLOCATED.load(Ordering::Relaxed);
    PEAK.store(current, Ordering::Relaxed);
}

/// Reset all tracking counters
pub fn reset_all() {
    ALLOCATED.store(0, Ordering::Relaxed);
    PEAK.store(0, Ordering::Relaxed);
    ALLOC_COUNT.store(0, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracking_basic() {
        reset_all();

        let stats_before = get_stats();
        let _data = vec![0u8; 1024]; // Allocate 1KB
        let stats_after = get_stats();

        assert!(stats_after.peak >= 1024, "Peak should include 1KB allocation");
        assert!(
            stats_after.alloc_count > stats_before.alloc_count,
            "Should count allocations"
        );
    }

    #[test]
    fn test_peak_tracking() {
        reset_all();

        {
            let _large = vec![0u8; 10 * 1024]; // 10KB
        } // Deallocated

        let peak = get_stats().peak;
        let current = get_stats().current;

        assert!(peak >= 10 * 1024, "Peak should remember 10KB");
        assert!(current < peak, "Current should be less after deallocation");
    }
}
