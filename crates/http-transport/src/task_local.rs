//! Task-local storage abstraction for WASI Preview 2 → Preview 3 migration.
//!
//! This module provides an abstraction layer over task-scoped state that works
//! in both Preview 2 (single-threaded, one task at a time) and will work in
//! Preview 3 (multi-threaded, multiple concurrent tasks).
//!
//! ## Preview 2 Implementation (Current)
//!
//! Uses `thread_local!` with `Mutex` for state storage. Since WASI P2 is
//! single-threaded with one active task per thread, this works correctly:
//! - thread_local state = current task's state
//! - Mutex provides defensive thread-safety (low overhead, ~5-10ns)
//!
//! ## Preview 3 Implementation (Future)
//!
//! Will use `context.get/set` built-ins for per-task state storage:
//! - Each task gets independent state on the heap
//! - Pointer stored in task's context-local storage (1 i32 slot)
//! - Runtime automatically switches context when switching tasks
//! - Zero changes required outside this module
//!
//! ## Migration Path
//!
//! When Preview 3 is released:
//! 1. Update `with_state()` to use `context::get()` instead of `thread_local!`
//! 2. Update `init_task()` to allocate heap state and call `context::set()`
//! 3. Update `cleanup_task()` to free heap state
//! 4. Zero changes needed in transport or protocol code
//!
//! See: https://github.com/WebAssembly/component-model/blob/main/design/mvp/Async.md#context-local-storage

use std::collections::HashMap;
use std::sync::Mutex;

use crate::bindings::wasi::io::streams::OutputStream;
use crate::bindings::wasmcp::mcp::protocol::ServerCapability;
use crate::MessageState;

/// Per-task state for the current HTTP request.
///
/// In P2: One instance per thread (stored in thread_local)
/// In P3: One instance per task (heap-allocated, pointer in context-local storage)
pub struct TaskState {
    /// The output stream for writing the HTTP response
    pub output_stream: Option<OutputStream>,

    /// Key-value context storage accessible to handlers via context::get/set
    pub context_store: HashMap<String, Vec<u8>>,

    /// Registered server capabilities for this request
    pub capabilities: Vec<ServerCapability>,

    /// Message lifecycle state for output interface
    pub message_state: MessageState,
}

impl TaskState {
    /// Create new task state with default values.
    fn new() -> Self {
        Self {
            output_stream: None,
            context_store: HashMap::new(),
            capabilities: Vec::new(),
            message_state: MessageState::NotStarted,
        }
    }

    /// Reset state for a new task.
    fn reset(&mut self) {
        self.output_stream = None;
        self.context_store.clear();
        self.capabilities.clear();
        self.message_state = MessageState::NotStarted;
    }
}

// =============================================================================
// Preview 2 Implementation (thread-local + Mutex)
// =============================================================================

thread_local! {
    /// Per-task state storage.
    ///
    /// P2: One per thread (safe because WASI is single-threaded)
    /// P3: Will be replaced with heap allocation + context.get/set
    static STATE: Mutex<TaskState> = Mutex::new(TaskState::new());
}

/// Access the current task's state.
///
/// This provides a uniform API that will work in both P2 and P3.
///
/// # Preview 2 Implementation
///
/// Accesses thread-local state (one task per thread).
///
/// # Preview 3 Implementation (Future)
///
/// Will retrieve state pointer from context-local storage:
/// ```ignore
/// let ptr = context::get() as *mut TaskState;
/// unsafe { f(&mut *ptr) }
/// ```
pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut TaskState) -> R,
{
    STATE.with(|state| {
        let mut guard = state.lock().unwrap();
        f(&mut guard)
    })
}

/// Initialize task state at the start of a new request.
///
/// Must be called before any handlers access the state.
///
/// # Preview 2 Implementation
///
/// Resets the thread-local state for the new request.
///
/// # Preview 3 Implementation (Future)
///
/// Will allocate new TaskState on heap and store pointer in context-local storage:
/// ```ignore
/// let state = Box::new(TaskState { output_stream: Some(stream), .. });
/// context::set(Box::into_raw(state) as i32);
/// ```
pub fn init_task(output_stream: OutputStream) {
    with_state(|state| {
        state.reset();
        state.output_stream = Some(output_stream);
    })
}

/// Clean up task state at the end of a request.
///
/// Should be called when the request completes to free resources.
///
/// # Preview 2 Implementation
///
/// Clears the thread-local state.
///
/// # Preview 3 Implementation (Future)
///
/// Will free heap-allocated state:
/// ```ignore
/// let ptr = context::get() as *mut TaskState;
/// if !ptr.is_null() {
///     unsafe { let _ = Box::from_raw(ptr); } // Drop and free
///     context::set(0);
/// }
/// ```
pub fn cleanup_task() {
    with_state(|state| {
        state.reset();
    })
}

// =============================================================================
// Preview 3 Implementation (commented out, for reference)
// =============================================================================

/*
// This will replace the P2 implementation when Preview 3 is released:

use crate::bindings::canon::context;  // P3 built-ins

pub fn with_state<F, R>(f: F) -> R
where
    F: FnOnce(&mut TaskState) -> R,
{
    let ptr = context::get() as *mut TaskState;

    // Lazy allocation if this is first access in task
    let ptr = if ptr.is_null() {
        let state = Box::new(TaskState::new());
        let ptr = Box::into_raw(state);
        context::set(ptr as i32);
        ptr
    } else {
        ptr
    };

    unsafe { f(&mut *ptr) }
}

pub fn init_task(output_stream: OutputStream) {
    let state = Box::new(TaskState {
        output_stream: Some(output_stream),
        context_store: HashMap::new(),
        capabilities: Vec::new(),
        message_state: MessageState::NotStarted,
    });
    context::set(Box::into_raw(state) as i32);
}

pub fn cleanup_task() {
    let ptr = context::get() as *mut TaskState;
    if !ptr.is_null() {
        unsafe {
            let _ = Box::from_raw(ptr);  // Drop and free
        }
        context::set(0);
    }
}
*/
