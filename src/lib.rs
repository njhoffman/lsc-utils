use std::process::ExitCode;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run_from_env() -> ExitCode {
    init_debug_tracing();
    // Staged: full CLI lands in step 4. For now, exit cleanly so the binary
    // is installable and the test harness has something to invoke.
    tracing::debug!("lsc {VERSION} scaffold invoked");
    ExitCode::SUCCESS
}

fn init_debug_tracing() {
    if std::env::var_os("DEBUG").is_some_and(|v| v == "1") {
        let filter = tracing_subscriber::EnvFilter::try_from_env("RUST_LOG")
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"));
        let _ = tracing_subscriber::fmt()
            .with_writer(std::io::stderr)
            .with_env_filter(filter)
            .try_init();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_non_empty() {
        assert!(!VERSION.is_empty());
    }
}
