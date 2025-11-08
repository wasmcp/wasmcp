//! Component resolution
//!
//! This module handles resolving component specifications to local file paths:
//! - Spec resolution (aliases, paths, registry packages)
//! - Framework component resolution (transport, server-io, session-store, method-not-found)
//! - Dependency downloading and caching

pub mod dependencies;
pub mod framework;
pub mod spec;

pub use dependencies::{DownloadConfig, PackageClient, download_dependencies, get_dependency_path};
pub use framework::{
    FrameworkComponent, resolve_framework_component, resolve_kv_store_component,
    resolve_method_not_found_component, resolve_server_io_component,
    resolve_session_store_component, resolve_transport_component,
};
pub use spec::resolve_component_spec;
