use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};
use tracing_appender::rolling;
use std::io;

pub fn init_logging() -> anyhow::Result<tracing_appender::non_blocking::WorkerGuard> {
    // Create logs directory if it doesn't exist
    std::fs::create_dir_all("logs")?;
    
    // Create a file appender that rotates daily
    let file_appender = rolling::daily("logs", "b2cli.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    
    // Create formatters
    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .json();
    
    let stdout_layer = fmt::layer()
        .with_writer(io::stdout)
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(false)
        .pretty();
    
    // Set up the subscriber with environment filters
    // File gets everything at debug level
    let file_filter = EnvFilter::try_from_env("FILE_LOG")
        .unwrap_or_else(|_| EnvFilter::new("debug"))
        .add_directive("sqlx=info".parse()?)
        .add_directive("b2cli=trace".parse()?)
        .add_directive("b2cli::file_scanner=trace".parse()?)
        .add_directive("b2cli::routes=trace".parse()?);
    
    // Console gets only info and above
    let console_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"))
        .add_directive("sqlx=warn".parse()?)
        .add_directive("hyper=warn".parse()?)
        .add_directive("tower=warn".parse()?)
        .add_directive("b2cli=info".parse()?);
    
    tracing_subscriber::registry()
        .with(file_layer.with_filter(file_filter))
        .with(stdout_layer.with_filter(console_filter))
        .init();
    
    tracing::info!("B2CLI logging initialized");
    tracing::info!("Log files are stored in ./logs directory");
    
    Ok(guard)
}