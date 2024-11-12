//! Tag management for EVM event processing.
//! 
//! This module provides thread-safe access to a global string tag that's used to
//! differentiate between different EVM contract instances during event processing.
//! The tag helps track and manage historical and live events for specific contracts.

use std::sync::OnceLock;

/// Global tag for contract event tracking with a default value of "_".
/// This tag is initialized once and remains constant throughout the lifecycle
/// of event processing to ensure consistent event tracking across restarts.
static TAG: OnceLock<String> = OnceLock::new();

pub fn get_tag() -> String {
    TAG.get().cloned().unwrap_or_else(|| String::from("_"))
}

pub fn set_tag(new_tag: impl Into<String>) -> Result<(), &'static str> {
    TAG.set(new_tag.into())
        .map_err(|_| "Tag has already been initialized")
}
