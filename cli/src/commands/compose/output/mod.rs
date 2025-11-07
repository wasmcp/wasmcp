//! User-facing output formatting
//!
//! This module handles formatting user-facing messages during and after
//! component composition, including pipeline diagrams and success messages.

pub mod formatting;

pub use formatting::{
    print_handler_pipeline_diagram, print_handler_success_message, print_pipeline_diagram,
    print_success_message,
};
