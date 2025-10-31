//! This file is the old monolithic serializer.rs that has been refactored
//! into the serializer/ module structure. It is kept for reference but
//! should be deleted once the refactoring is verified to work.
//!
//! The new structure is:
//! - serializer/mod.rs - Module declarations and public API exports
//! - serializer/types.rs - Shadow types for JSON serialization
//! - serializer/content.rs - Content block serialization
//! - serializer/responses.rs - Server response serialization