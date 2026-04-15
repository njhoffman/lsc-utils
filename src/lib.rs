use std::io::{self, IsTerminal, Write};
use std::path::Path;
use std::process::ExitCode;

use anyhow::Result;

pub mod cli;
pub mod config;
pub mod fs;
pub mod options;
pub mod render;
pub mod util;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const FALLBACK_TERMINAL_WIDTH: usize = 80;

pub fn run_from_env() -> ExitCode {
    init_debug_tracing();
    let args = cli::Args::parse_argv();
    match run(args) {
        Ok(code) => code,
        Err(err) => {
            let _ = writeln!(io::stderr(), "lsc: {err:#}");
            ExitCode::from(1)
        }
    }
}

pub fn run(args: cli::Args) -> Result<ExitCode> {
    let opts = args.resolve()?;
    let cfg = config::Config::load(config::Config::default_user_dir().as_deref())?;
    let theme = config::Theme::from_config(&cfg, opts.theme)?;
    let icons = config::Icons::from_config(&cfg)?;

    let stdout = io::stdout();
    let mut handle = stdout.lock();

    let multi_target = opts.paths.len() > 1;
    for (i, path) in opts.paths.iter().enumerate() {
        if multi_target {
            if i > 0 {
                writeln!(handle)?;
            }
            writeln!(handle, "{}:", path.display())?;
        }
        render_path(&opts, &theme, &icons, path, &mut handle)?;
    }
    Ok(ExitCode::SUCCESS)
}

fn render_path(
    opts: &options::RunOptions,
    theme: &config::Theme,
    icons: &config::Icons,
    path: &Path,
    out: &mut dyn Write,
) -> Result<()> {
    let entries = if path.is_dir() {
        fs::scan_directory(path, &opts.filter)?
    } else {
        vec![fs::entry::from_user_path(path)?]
    };

    // Step 4 sort: stable name order (case-sensitive, ASCII-ish). Locale-aware
    // sorting + the full --sort matrix land in step 6.
    let mut sorted = entries;
    sorted.sort_by(|a, b| a.name.cmp(&b.name));

    if matches!(opts.layout, options::LayoutMode::Long) {
        let (counts, _) = render::long::render(
            &sorted,
            theme,
            icons,
            opts.color_mode,
            opts.show_icons,
            &opts.long,
            out,
        )?;
        if let Some(kind) = opts.report {
            let txt = render::long::render_report(&counts, kind, theme, opts.color_mode);
            out.write_all(txt.as_bytes())?;
        }
        return Ok(());
    }

    let builder = render::CellBuilder {
        theme,
        icons,
        color_mode: opts.color_mode,
        show_icons: opts.show_icons,
    };
    let cells = render::build_cells(&sorted, &builder);

    match opts.layout {
        options::LayoutMode::OnePerLine => render::one_per_line::render(&cells, out)?,
        options::LayoutMode::Vertical => {
            let w = terminal_width(out);
            render::grid::render_grid(&cells, w, render::grid::GridKind::Vertical, out)?;
        }
        options::LayoutMode::Horizontal => {
            let w = terminal_width(out);
            render::grid::render_grid(&cells, w, render::grid::GridKind::Horizontal, out)?;
        }
        options::LayoutMode::Long => unreachable!("handled above"),
    }
    Ok(())
}

/// Terminal width detection: prefer COLUMNS env var (lets tests pin width
/// without a tty), then `terminal_size`, then fall back.
fn terminal_width(out: &dyn Write) -> usize {
    if let Some(w) = std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
    {
        if w > 0 {
            return w;
        }
    }
    let _ = out; // future: detect dimensions of the actual writer if it's a tty
    if io::stdout().is_terminal() {
        if let Some((terminal_size::Width(w), _)) = terminal_size::terminal_size() {
            return w as usize;
        }
    }
    FALLBACK_TERMINAL_WIDTH
}

fn init_debug_tracing() {
    if std::env::var_os("DEBUG").is_some_and(|v| v == "1") {
        let filter = tracing_subscriber::EnvFilter::try_from_env("RUST_LOG")
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("debug"));
        let _ = tracing_subscriber::fmt()
            .with_writer(io::stderr)
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
