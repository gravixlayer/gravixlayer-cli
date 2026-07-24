// src/output/mod.rs — Unified output helpers.
//
// All command handlers print through the functions in this module, which
// respect the global `--output` flag:
//
//   table  (default)  — human-readable comfy-table or plain text
//   json              — raw JSON for scripting / pipe
//   quiet             — suppress all output except errors

pub mod table;

use std::fmt;

use crate::cli::OutputFormat;

// ---------------------------------------------------------------------------
// Spinner (progress indicator)
// ---------------------------------------------------------------------------

/// A simple indeterminate spinner backed by indicatif.
///
/// Automatically calls `.finish_and_clear()` on drop so callers don't need to
/// manually clean up in error paths.
pub struct Spinner {
    pb: indicatif::ProgressBar,
}

impl Spinner {
    pub fn new(message: impl Into<String>) -> Self {
        let pb = indicatif::ProgressBar::new_spinner();
        pb.set_style(
            indicatif::ProgressStyle::default_spinner()
                .tick_chars("⣾⣽⣻⢿⡿⣟⣯⣷")
                .template("{spinner:.cyan} {msg}")
                .unwrap(),
        );
        pb.enable_steady_tick(std::time::Duration::from_millis(80));
        pb.set_message(message.into());
        Self { pb }
    }

    pub fn set_message(&self, msg: impl Into<String>) {
        self.pb.set_message(msg.into());
    }

    pub fn finish_ok(&self, msg: impl Into<String>) {
        self.pb.finish_and_clear();
        println!("\u{2714} {}", msg.into());
    }

    #[allow(dead_code)]
    pub fn finish_err(&self, msg: impl Into<String>) {
        self.pb.finish_and_clear();
        eprintln!("\u{2716} {}", msg.into());
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.pb.finish_and_clear();
    }
}

// ---------------------------------------------------------------------------
// Output helpers
// ---------------------------------------------------------------------------

/// Print a value as JSON (pretty-printed) or human text, depending on the
/// output mode.
///
/// `table_fn` is called to render human-readable output.  For `quiet` mode
/// both are suppressed.
pub fn print_or_json<T, F>(mode: OutputFormat, value: &T, table_fn: F)
where
    T: serde::Serialize,
    F: FnOnce(),
{
    match mode {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_else(|e| format!("{e}"))
            );
        }
        OutputFormat::Table => table_fn(),
        OutputFormat::Quiet => {}
    }
}

/// Print a plain status message (suppressed in json/quiet mode).
pub fn info(mode: OutputFormat, msg: impl fmt::Display) {
    if mode == OutputFormat::Table {
        println!("{msg}");
    }
}

/// Print a success message (suppressed in json/quiet mode).
pub fn success(mode: OutputFormat, msg: impl fmt::Display) {
    if mode == OutputFormat::Table {
        println!("\u{2714} {msg}");
    }
}

/// Print a warning to stderr regardless of output mode.
pub fn warn(msg: impl fmt::Display) {
    eprintln!("\u{26a0}  {msg}");
}

/// Print an error to stderr regardless of output mode.
pub fn error(msg: impl fmt::Display) {
    eprintln!("\u{2716} {msg}");
}

/// Print a key-value pair in `  key: value` style.
pub fn kv(mode: OutputFormat, key: &str, value: impl fmt::Display) {
    if mode == OutputFormat::Table {
        println!("  {key:<24} {value}");
    }
}
