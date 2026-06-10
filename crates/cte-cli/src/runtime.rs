use tracing_subscriber::{fmt, prelude::*, EnvFilter};

pub fn init_logging(log_file: &str, verbosity: u8) -> anyhow::Result<()> {
    let level = match verbosity {
        0 => "info",
        1 => "debug",
        _ => "trace",
    };

    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{level},hyper=warn,reqwest=warn,tungstenite=warn")));

    let log_dir = std::path::Path::new(log_file)
        .parent()
        .unwrap_or(std::path::Path::new("./logs"));
    std::fs::create_dir_all(log_dir)?;

    let file_name = std::path::Path::new(log_file)
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or("cte.log");

    let file_appender = tracing_appender::rolling::daily(log_dir, file_name);
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    // We leak the guard so it lives for the program duration
    std::mem::forget(_guard);

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::layer()
                .json()
                .with_writer(non_blocking)
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .with(
            fmt::layer()
                .with_writer(std::io::stderr)
                .with_target(false)
                .compact(),
        )
        .init();

    Ok(())
}
