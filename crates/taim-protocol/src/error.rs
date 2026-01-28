//! Error types for the taim-protocol crate.
//!
//! This module defines all error types that can occur when working with
//! protocol types, including serialization failures and validation errors.

use thiserror::Error;

/// Errors that can occur during protocol operations.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// Failed to serialize a protocol type to JSON.
    #[error("failed to serialize to JSON: {0}")]
    SerializationFailed(#[source] serde_json::Error),

    /// Failed to deserialize a protocol type from JSON.
    #[error("failed to deserialize from JSON: {0}")]
    DeserializationFailed(#[source] serde_json::Error),

    /// A task with the given ID was not found.
    #[error("task not found: {0}")]
    TaskNotFound(uuid::Uuid),

    /// The specified lane does not exist on the board.
    #[error("lane not found: {0:?}")]
    LaneNotFound(crate::board::LaneKind),

    /// A task title was empty or invalid.
    #[error("invalid task title: title cannot be empty")]
    InvalidTaskTitle,
}

/// A specialized Result type for protocol operations.
pub type Result<T> = std::result::Result<T, ProtocolError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = ProtocolError::InvalidTaskTitle;
        assert_eq!(err.to_string(), "invalid task title: title cannot be empty");

        let task_id = uuid::Uuid::new_v4();
        let err = ProtocolError::TaskNotFound(task_id);
        assert!(err.to_string().contains("task not found"));
    }
}
