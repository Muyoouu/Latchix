use crate::error::AppError;

/// Applies embedded migrations.
pub fn run() -> Result<(), AppError> {
    Err(AppError::NotImplemented("migrate"))
}
