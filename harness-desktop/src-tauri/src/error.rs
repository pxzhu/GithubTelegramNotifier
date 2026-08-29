use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HarnessError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("validation error: {0}")]
    Validation(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("operation unavailable: {0}")]
    Unavailable(String),
    #[error("permission denied: {0}")]
    PermissionDenied(String),
}

pub type HarnessResult<T> = Result<T, HarnessError>;

#[derive(Debug, Clone, Serialize)]
pub struct CommandError {
    pub code: &'static str,
    pub message: String,
}

impl From<HarnessError> for CommandError {
    fn from(value: HarnessError) -> Self {
        let code = match &value {
            HarnessError::Database(_) => "database_error",
            HarnessError::Io(_) => "io_error",
            HarnessError::Serialization(_) => "serialization_error",
            HarnessError::Validation(_) => "validation_error",
            HarnessError::NotFound(_) => "not_found",
            HarnessError::Conflict(_) => "conflict",
            HarnessError::Unavailable(_) => "unavailable",
            HarnessError::PermissionDenied(_) => "permission_denied",
        };
        Self {
            code,
            message: value.to_string(),
        }
    }
}

impl From<std::sync::PoisonError<std::sync::MutexGuard<'_, crate::database::Database>>>
    for CommandError
{
    fn from(
        _: std::sync::PoisonError<std::sync::MutexGuard<'_, crate::database::Database>>,
    ) -> Self {
        Self {
            code: "state_poisoned",
            message: "database state is unavailable".into(),
        }
    }
}
