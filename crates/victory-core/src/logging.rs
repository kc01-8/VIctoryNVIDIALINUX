//! One tracing setup for every binary.
//!
//! Filter with `VICTORY_LOG` (falls back to `RUST_LOG`), e.g.
//! `VICTORY_LOG=victory_loader=trace,info`. Debug and trace output exist
//! only in builds with the `diagnostics` feature; a plain release build
//! logs info and above whatever the filter asks for.
//!
//! `VICTORY_LOG_FILE=<path>` appends to that file what would go to stderr,
//! for launchers that do not hand the process a console (Proton
//! Experimental). On Windows the process's stderr itself is pointed there,
//! so crash reports and stack dumps printed with `eprintln!` land in it
//! too; elsewhere the log lines alone are written to it.

use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

/// The most verbose level this build can log.
const CAP: LevelFilter = if cfg!(feature = "diagnostics") {
    LevelFilter::TRACE
} else {
    LevelFilter::INFO
};

pub fn init(default_level: &str) {
    let filter = std::env::var("VICTORY_LOG")
        .or_else(|_| std::env::var("RUST_LOG"))
        .ok()
        .and_then(|v| EnvFilter::try_new(v).ok())
        .unwrap_or_else(|| EnvFilter::new(default_level));
    let file = std::env::var_os("VICTORY_LOG_FILE").and_then(|path| {
        std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .inspect_err(|e| eprintln!("VICTORY_LOG_FILE {}: {e}", path.to_string_lossy()))
            .ok()
    });
    // Stderr pointed at the file: one stream for the log and for what is
    // printed directly. Otherwise (or where that fails) a layer of its own.
    let (redirected, file) = match file {
        Some(file) => match redirect_stderr(file) {
            Ok(()) => (true, None),
            Err(file) => (false, Some(file)),
        },
        None => (false, None),
    };
    let _ = tracing_subscriber::registry()
        .with(CAP)
        .with(filter)
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(!redirected)
                .with_writer(std::io::stderr),
        )
        .with(file.map(|file| {
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_ansi(false)
                .with_writer(std::sync::Mutex::new(file))
        }))
        .try_init();
}

/// Makes `file` the process's stderr; gives it back if that fails.
#[cfg(windows)]
fn redirect_stderr(file: std::fs::File) -> Result<(), std::fs::File> {
    use std::os::windows::io::{AsRawHandle, IntoRawHandle};
    use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};
    // SAFETY: a valid handle of ours; on success the process keeps it open
    // as its stderr for as long as it runs.
    if unsafe { SetStdHandle(STD_ERROR_HANDLE, file.as_raw_handle()) } != 0 {
        let _ = file.into_raw_handle();
        Ok(())
    } else {
        Err(file)
    }
}

#[cfg(not(windows))]
fn redirect_stderr(file: std::fs::File) -> Result<(), std::fs::File> {
    Err(file)
}
