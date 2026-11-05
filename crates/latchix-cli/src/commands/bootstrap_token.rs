use crate::cli::BootstrapTokenArgs;
use crate::error::AppError;

/// Prints a one-time bootstrap token.
pub fn run(args: BootstrapTokenArgs) -> Result<(), AppError> {
    let _ = (args.ttl_seconds, args.json);
    Err(AppError::NotImplemented("bootstrap-token"))
}
