//! Definitions of errors returned by the application.

/// Errors that can occur in the application
// TODO: Add more specific errors, implement thiserror or similar crate
#[derive(Debug)]
pub enum Error {
    /// Invalid network builder configuration
    BuilderConfig(String),
}
