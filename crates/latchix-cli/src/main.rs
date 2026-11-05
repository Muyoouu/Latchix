#![forbid(unsafe_code)]

mod cli;
mod commands;
mod config;
mod error;
mod logging;

use std::process::ExitCode;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::error::AppError;

fn main() -> ExitCode {
    let cli = Cli::parse();
    logging::init(cli.log_format, cli.quiet);

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build();

    let result = match runtime {
        Ok(runtime) => runtime.block_on(run(cli)),
        Err(error) => Err(AppError::Io(error)),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("latchix: {error}");
            ExitCode::from(error.exit_code())
        }
    }
}

async fn run(cli: Cli) -> Result<(), AppError> {
    let config_path = cli.config;

    match cli.command {
        Command::Serve(args) => commands::serve::run(&config_path, args).await,
        Command::Migrate => commands::migrate::run(),
        Command::Backup(args) => commands::backup::run(args),
        Command::Health => commands::health::run(&config_path).await,
        Command::BootstrapToken(args) => commands::bootstrap_token::run(args),
        Command::Doctor => commands::doctor::run(&config_path),
    }
}
