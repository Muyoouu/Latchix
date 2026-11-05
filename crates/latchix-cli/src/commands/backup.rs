use crate::cli::BackupArgs;
use crate::error::AppError;

/// Writes a consistent backup.
pub fn run(args: BackupArgs) -> Result<(), AppError> {
    let _ = args.output;
    Err(AppError::NotImplemented("backup"))
}
