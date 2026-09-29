//! Typed failures for tag server functions, mapped into Leptos `ServerFnError`.
//!
//! Keep domain failures as [`TagServerError`] while assembling Higgs/Valence work.
//! At the last step before returning from a `#[server]` fn, call [`into_server_error`]
//! so the client sees a safe string and `tracing` records the failure once.

use leptos::prelude::ServerFnError;
use thiserror::Error;

/// Stable code carried by the [`TagServerError::DuplicateName`] message.
pub const DUPLICATE_NAME_CODE: &str = "tag.duplicate_name";

/// Whether a server fn failed because the tag name is already taken.
#[must_use]
pub fn is_duplicate_name(err: &ServerFnError) -> bool {
    err.to_string().contains(DUPLICATE_NAME_CODE)
}

/// Tag-app server function errors (auth, domain, search sources, infrastructure).
///
/// Classification:
/// - [`NotAuthenticated`](Self::NotAuthenticated) / [`AccessDenied`](Self::AccessDenied) /
///   [`NotFound`](Self::NotFound) / [`DuplicateName`](Self::DuplicateName) /
///   [`InvalidSource`](Self::InvalidSource) — permanent for this request
/// - [`Service`](Self::Service) / [`SearchSource`](Self::SearchSource) /
///   [`Valence`](Self::Valence) — infrastructure / unexpected
#[derive(Error, Debug)]
pub enum TagServerError {
    /// The caller has no authenticated session but one is required.
    #[error("Authentication required")]
    NotAuthenticated,

    /// No tag row for the given bare id.
    #[error("tag not found: {id}")]
    NotFound {
        /// Bare Valence record id (safe to log).
        id: String,
    },

    /// Caller failed an ownership / privacy check.
    #[error("Access denied by policy: {policy}")]
    AccessDenied {
        /// Policy rule name (e.g. `tag::TAG_DATA_OWNER`).
        policy: &'static str,
    },

    /// Another tag already uses this name (ignoring case and surrounding
    /// whitespace). The message starts with [`DUPLICATE_NAME_CODE`] so clients
    /// can detect it with [`is_duplicate_name`].
    #[error("{DUPLICATE_NAME_CODE}: a tag named {name_key} already exists")]
    DuplicateName {
        /// The requested name after `tag::normalize_name_key`.
        name_key: String,
    },

    /// Picker requested a search source this server fn does not serve.
    #[error("unsupported search source: {0}")]
    InvalidSource(String),

    /// A registered search source failed while answering a catalog search.
    #[error("tag catalog search failed: {0}")]
    SearchSource(String),

    /// Underlying Valence, history, or ownership service failure from `tag`.
    #[error("tag catalog {operation} failed: {message}")]
    Service {
        /// Operation label (e.g. `create`, `delete`, `list`).
        operation: &'static str,
        /// Display-safe source message (no secrets).
        message: String,
    },

    /// Failed to build a Valence handle from Higgs.
    #[error("Failed to build Valence: {0}")]
    Valence(String),
}

#[cfg(feature = "ssr")]
impl From<tag::TagError> for TagServerError {
    fn from(value: tag::TagError) -> Self {
        match value {
            tag::TagError::NotFound { id } => Self::NotFound { id },
            tag::TagError::DuplicateName { name_key } => Self::DuplicateName { name_key },
            tag::TagError::AccessDenied { policy } => Self::AccessDenied { policy },
            tag::TagError::Service { operation, source } => Self::Service {
                operation,
                message: source.to_string(),
            },
        }
    }
}

/// Map [`TagServerError`] to Leptos [`ServerFnError`] and log once at the boundary.
///
/// Call this once at the UI/server-fn boundary
/// (`result.map_err(|e| into_server_error("list_tags", e))`). Under `ssr`, emits a
/// `tracing::warn` with low-cardinality `operation` and `error_kind` labels.
///
/// Log at this boundary only; do not re-log while propagating the same failure.
#[allow(clippy::needless_pass_by_value)]
pub fn into_server_error(operation: &'static str, e: TagServerError) -> ServerFnError {
    let error_kind = match &e {
        TagServerError::NotAuthenticated => "not_authenticated",
        TagServerError::NotFound { .. } => "not_found",
        TagServerError::DuplicateName { .. } => "duplicate_name",
        TagServerError::AccessDenied { .. } => "access_denied",
        TagServerError::InvalidSource(_) => "invalid_source",
        TagServerError::SearchSource(_) => "search_source",
        TagServerError::Service { .. } => "service",
        TagServerError::Valence(_) => "valence",
    };

    #[cfg(feature = "ssr")]
    {
        if matches!(e, TagServerError::DuplicateName { .. }) {
            // Expected user outcome; the message echoes the typed name.
            tracing::debug!(
                operation,
                error_kind,
                "tag server fn rejected duplicate name"
            );
        } else {
            tracing::warn!(
                operation,
                error_kind,
                error = %e,
                "tag server fn failed"
            );
        }
    }
    #[cfg(not(feature = "ssr"))]
    {
        let _ = (operation, error_kind);
    }

    ServerFnError::new(e.to_string())
}

#[cfg(test)]
mod tests {
    use super::{into_server_error, is_duplicate_name, TagServerError};

    #[test]
    fn create_tag_duplicate_carries_stable_code_sad() {
        let err = into_server_error(
            "create_tag",
            TagServerError::DuplicateName {
                name_key: "ops".into(),
            },
        );
        assert!(is_duplicate_name(&err));
        assert!(err.to_string().contains("a tag named ops already exists"));
        let other = into_server_error("create_tag", TagServerError::NotAuthenticated);
        assert!(!is_duplicate_name(&other));
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn duplicate_name_error_maps_in_tag_app_sad() {
        let mapped: TagServerError = tag::TagError::DuplicateName {
            name_key: "ops".into(),
        }
        .into();
        assert!(
            matches!(mapped, TagServerError::DuplicateName { ref name_key } if name_key == "ops")
        );
    }

    #[test]
    fn maps_not_found_display() {
        let err = into_server_error("get_tag", TagServerError::NotFound { id: "abc".into() });
        let msg = err.to_string();
        assert!(msg.contains("tag not found"));
        assert!(msg.contains("abc"));
    }

    #[test]
    fn maps_access_denied_display() {
        let err = into_server_error(
            "update_tag",
            TagServerError::AccessDenied {
                policy: "tag::TAG_DATA_OWNER",
            },
        );
        assert!(err.to_string().contains("Access denied"));
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn from_tag_error_preserves_variants() {
        let denied: TagServerError = tag::TagError::AccessDenied {
            policy: "tag::TAG_DATA_OWNER",
        }
        .into();
        assert!(matches!(
            denied,
            TagServerError::AccessDenied {
                policy: "tag::TAG_DATA_OWNER"
            }
        ));
    }
}
