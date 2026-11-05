#![forbid(unsafe_code)]

/// Name of the Cargo feature that enables the admin UI.
pub const FEATURE_NAME: &str = "ui";

/// Returns whether this build contains the opt-in admin UI.
#[must_use]
pub const fn is_compiled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui_crate_is_available_when_compiled() {
        assert!(super::is_compiled());
    }
}
