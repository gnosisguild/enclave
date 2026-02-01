use tracing::error;

/// Formats panic errors so they are seen in logs clearly
pub fn major_issue(msg: &str, e: impl Into<anyhow::Error>) -> String {
    error!("\n\n\nMAJOR ISSUE: {msg}.\n\nThe error supplied was: {:?}\n\n As a precaution we are crashing the system.\n\n\n", e.into());
    format!("System has crashed. Nothing personal. Goodbye.")
}
