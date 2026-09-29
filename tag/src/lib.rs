//! Shared tag catalog — flat labels with optional taxonomy and audit history.
//!
//! `tag` is a Valence catalog of reusable labels (`Tag`) plus their per-field
//! change trail. Product records attach tags through their own Valence
//! `ManyToMany` connections, not through this crate. The Leptos admin UI lives
//! in sibling crate `tag-app`.
//!
//! ## Features
//!
//! - **Tag catalog service** — Create, update, delete, get, and list shared
//!   labels with automatic `tag_history` rows on every mutation. Call from SSR
//!   services or Higgs server fns once Valence is available.
//!   [Get started](#tag-catalog-crud)
//! - **Per-field change history** — [`side_effects::TagHistoryWriter`] appends
//!   one `tag_history` row per changed field so detail pages can embed a
//!   platform timeline.
//! - **Unique names** — Names are unique ignoring case and surrounding
//!   spaces. The `name_key` column carries a unique index, so a duplicate
//!   create or rename fails with [`TagError::DuplicateName`] even when two
//!   requests race. [`get_by_name`] looks a tag up the same way.
//! - **Ownership-gated delete** — Only the tag owner or System may delete a
//!   row; see [`privacy_policies`].
//! - **Catalog search source** — [`search_sources::TagCatalogSearchSource`]
//!   feeds Orbital pickers; `tag-app` wraps it in `TagCatalogPicker`.
//!
//! ## Tag catalog CRUD
//!
//! The catalog service is the steady-state API for label rows. Each write
//! appends history through [`side_effects::TagHistoryWriter`] so audit
//! timelines stay complete without separate history calls. Use it from
//! `tag-app` server fns or custom SSR handlers once session Valence is wired.
//!
//! **Prerequisites:** `ssr` feature; a `Valence` handle whose router includes
//! the tag schemas (compiled in with this crate under `ssr`).
//!
//! ```rust,ignore
//! use tag::types::{TagCreateInput, TagUpdateInput};
//! use tag::{create, delete, get, get_by_name, list, update, TagError};
//!
//! let created = create(
//!     TagCreateInput {
//!         name: "Office Supplies".into(),
//!         taxonomy: Some("spend".into()),
//!         description: None,
//!     },
//!     &valence,
//! ).await?;
//!
//! // Names are unique ignoring case and surrounding spaces.
//! let dup = create(
//!     TagCreateInput {
//!         name: " office supplies ".into(),
//!         taxonomy: None,
//!         description: None,
//!     },
//!     &valence,
//! ).await;
//! assert!(matches!(dup, Err(TagError::DuplicateName { .. })));
//! let found = get_by_name("OFFICE SUPPLIES", &valence).await?;
//! assert_eq!(found.map(|t| t.id), Some(created.id.clone()));
//!
//! let updated = update(
//!     &created.id,
//!     TagUpdateInput {
//!         name: None,
//!         taxonomy: Some("custom".into()),
//!         description: None,
//!     },
//!     &valence,
//! ).await?;
//! assert_eq!(updated.taxonomy.as_deref(), Some("custom"));
//!
//! let round_tripped = get(&created.id, &valence).await?.expect("tag exists");
//! assert_eq!(round_tripped.taxonomy.as_deref(), Some("custom"));
//!
//! let rows = list(&valence, None, Some("custom".into())).await?;
//! assert!(rows.iter().any(|r| r.id == created.id));
//!
//! delete(&created.id, &valence).await?;
//! ```
//!
//! On success `get` returns the mutated DTO, `list` includes matching taxonomy
//! filters, and `delete` removes the row after writing a final history entry.
//! [`TagError::NotFound`] surfaces on update of a missing id;
//! [`TagError::DuplicateName`] when `create` or `update` would reuse another
//! tag's name; [`TagError::AccessDenied`] when a non-owner calls `delete`; Valence and
//! history failures map to [`TagError::Service`]. `tag-app` server fns map
//! these into `ServerFnError`. Direct model privacy denials remain
//! `valence::Error` strings (for example pending deletion).
//!
//! Detail pages in `tag-app` embed
//! `<HistoryTimeline source=RecordId::new("tag", id) />` for the appended
//! history. Product schemas declare tag edges as `ManyToMany` on their own
//! `valence_schema!` (see [README.md](../README.md)).
//!
//! ## Feature flags
//!
//! | Flag | Effect |
//! |------|--------|
//! | `ssr` | Valence CRUD, history side effects, search source, and generated models. |
//! | `db-sqlite` | `SQLite` Valence backend (enabled by `ssr`). |
//! | `db-hybrid` | Hybrid Valence backend instead of default `SQLite` routing. |
//!
//! ## Examples
//!
//! Catalog create/update/list/delete with history:
//! [Tag catalog CRUD](#tag-catalog-crud).
//!
//! Run `cargo test -p tag --test tag_crud_contract`,
//! `cargo test -p tag --test tag_service_integration`, and
//! `cargo test -p tag --features ssr --test tag_unique_name` (duplicate names,
//! including concurrent creates).
//!
//! Workspace example `protected-tag-host` covers auth, catalog CRUD, and
//! `ManyToMany` composition (inventory `tag` / `/tag`):
//! `cargo run -p protected-tag-host`.

pub mod types;

#[cfg(feature = "ssr")]
pub mod embedded_surreal;

#[cfg(feature = "ssr")]
pub mod generated;

#[cfg(feature = "ssr")]
mod schemas;

#[cfg(feature = "ssr")]
pub mod privacy_policies;

#[cfg(feature = "ssr")]
pub mod side_effects;

pub mod search_sources;

#[cfg(feature = "ssr")]
pub mod tag;

#[cfg(feature = "ssr")]
pub use tag::*;

pub use search_sources::TagSearchSourceId;
pub use types::normalize_name_key;
