//! DTO contracts for the tag service / server function boundary.
//!
//! These types are the stable wire shapes between [`crate::tag`] CRUD (or the
//! `tag-app` server functions that call it) and UI callers; they intentionally hide
//! the generated Valence model types.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// The key tag names are unique on: `name` trimmed and lowercased.
///
/// `" Ops "`, `"ops"` and `"OPS"` all normalize to `"ops"`, so only one of
/// them can exist in the catalog.
#[must_use]
pub fn normalize_name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

/// Input for creating a new tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagCreateInput {
    /// Display name of the tag.
    pub name: String,
    /// Optional taxonomy/category grouping.
    pub taxonomy: Option<String>,
    /// Optional free-text description.
    pub description: Option<String>,
}

/// Partial update for an existing tag; only `Some` fields are applied.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagUpdateInput {
    /// New display name, if changed.
    pub name: Option<String>,
    /// New taxonomy/category grouping, if changed.
    pub taxonomy: Option<String>,
    /// New free-text description, if changed.
    pub description: Option<String>,
}

/// Tag row shape used in list views (list/table/picker).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagRowDto {
    /// Bare Valence record id (no table prefix).
    pub id: String,
    /// Display name of the tag.
    pub name: String,
    /// Optional taxonomy/category grouping.
    pub taxonomy: Option<String>,
    /// Optional free-text description.
    pub description: Option<String>,
    /// Human-readable owner label (`"you"`, a user id, or `"owner"` as a fallback).
    pub owner_display: String,
    /// Last-updated timestamp.
    pub updated_at: DateTime<Utc>,
}

/// Full tag row shape used on the detail page.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagDetailDto {
    /// Bare Valence record id (no table prefix).
    pub id: String,
    /// Display name of the tag.
    pub name: String,
    /// Optional taxonomy/category grouping.
    pub taxonomy: Option<String>,
    /// Optional free-text description.
    pub description: Option<String>,
    /// Human-readable owner label (`"you"`, a user id, or `"owner"` as a fallback).
    pub owner_display: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last-updated timestamp.
    pub updated_at: DateTime<Utc>,
}
