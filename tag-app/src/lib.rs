#![recursion_limit = "256"]
//! Tag catalog app — list, create, edit tags with embedded audit timeline.
//!
//! Orbital UI on top of the [`tag`] domain crate. Registers under `/tag` and
//! requires an authenticated, verified session
//! ([`uf_product::routes::RequireAuthenticated`]) before rendering.
//!
//! ## Features
//!
//! - **Tag admin routes** — Nested `/tag` routes behind an authenticated,
//!   verified guard for list, create, and detail/edit pages. Mount once when
//!   the host router starts. [Get started](#mount-tag-routes)
//! - **Tag catalog picker** — Multi-select [`TagCatalogPicker`] for connection
//!   UIs that attach shared catalog rows without reimplementing search-source
//!   wiring. [Get started](#embed-tag-catalog-picker)
//! - **Catalog pages** — [`TagListPage`], [`TagCreatePage`], and
//!   [`TagDetailPage`] with embedded
//!   [`HistoryTimeline`](record_history_leptos::HistoryTimeline) on detail.
//! - **Server function wrappers** — [`mod@server`] Higgs `#[server]` fns that
//!   map [`tag::TagError`] through [`TagServerError`] /
//!   [`into_server_error`] into `ServerFnError`.
//!
//! Hosts supply session chrome and identity. Schemas, catalog service rules,
//! and history writers stay in `tag`.
//!
//! ## Mount tag routes
//!
//! [`TagRoutes`] is the route tree hosts nest inside their Leptos `<Routes>`.
//! It wraps every tag page in [`TagVerifiedGuardRouteView`] so anonymous
//! sessions never render catalog UI. Mount during host router setup at
//! startup, alongside other `uf_app!` product routes — the macro registers
//! launcher metadata and the `/tag` inventory entry.
//!
//! **Prerequisites:** `ssr` (and `hydrate` matching the host); authenticated
//! session context from `uf_product`; Valence wired for the tag catalog.
//!
//! 1. Depend on `tag-app` with `ssr` / `hydrate` aligned to the host.
//! 2. Mount `<TagRoutes />` under the host `<Routes>`.
//!
//! ```rust,ignore
//! use tag_app::TagRoutes;
//! use leptos::prelude::*;
//! use leptos_router::components::Routes;
//!
//! view! {
//!     <Routes fallback=|| view! { <p>"not found"</p> }>
//!         <TagRoutes />
//!     </Routes>
//! }
//! ```
//!
//! On success `/tag` resolves to the catalog list, `/tag/create` opens the
//! create form, and `/tag/:id` renders detail with an embedded history
//! timeline. Unauthenticated sessions are rejected inside the verified guard.
//! Server fns require a session — see root `SECURITY.md`.
//!
//! Next: [Embed tag catalog picker](#embed-tag-catalog-picker), or domain APIs
//! in `tag`.
//!
//! ## Embed tag catalog picker
//!
//! [`TagCatalogPicker`] combines Orbital `TagPicker` with Tag's registered
//! catalog search source so product forms can multi-select shared labels.
//! Embed on any authenticated page where you store tag ids on a connection or
//! association field.
//!
//! Bind `selected` to your current id list and wire `on_change` to persist
//! selections through your product's connection API. Pass `seed` with the tags
//! already attached so their chips show names before the first search returns.
//!
//! **Prerequisites:** `ssr` on this crate for
//! [`search_tag_catalog`](server::search_tag_catalog); `hydrate` when the
//! picker runs in the browser.
//!
//! ```rust,ignore
//! use tag_app::TagCatalogPicker;
//! use leptos::prelude::*;
//!
//! let selected = RwSignal::new(Vec::<String>::new());
//! let on_change = Callback::new(move |ids: Vec<String>| {
//!     selected.set(ids);
//! });
//!
//! view! {
//!     <TagCatalogPicker
//!         selected=selected
//!         on_change=on_change
//!     />
//! }
//!
//! // Host persists `ids` from `on_change`; selection starts empty until picks land.
//! assert!(selected.get().is_empty());
//! ```
//!
//! On success the picker searches the catalog through
//! [`search_tag_catalog`](server::search_tag_catalog) as the user types and
//! reflects picks in `selected`. `manage_tags_href` defaults to `/tag` for
//! admins who need the full catalog UI. See [`TagCatalogPicker`] rustdoc for
//! prop contracts.
//!
//! ### Create a tag from the picker
//!
//! When no tag has the typed name, the list offers `<text> (new)`. Choosing it
//! opens [`TagCreateDialog`] with Name prefilled; Create adds the tag to the
//! catalog and selects it. The parent form still decides when to save the
//! selection. If someone else claimed the name first, the dialog offers "Use
//! existing tag" instead.
//!
//! Filters should turn this off, since a brand-new tag matches nothing:
//!
//! ```rust,ignore
//! view! { <TagCatalogPicker selected=filter_tags on_change=on_filter allow_create=false /> }
//! ```
//!
//! ## Feature flags
//!
//! | Flag | Effect |
//! |------|--------|
//! | `ssr` | Server-side Leptos split; required for catalog server fns. |
//! | `hydrate` | Client-side hydration for routed pages and the picker. |
//! | `preview` | Component preview surfaces for Orbital catalog. |
//!
//! ## Examples
//!
//! Mount [`TagRoutes`] per [Mount tag routes](#mount-tag-routes). Embed
//! [`TagCatalogPicker`] per [Embed tag catalog picker](#embed-tag-catalog-picker).
//!
//! Catalog CRUD and history writers live in sibling crate `tag`; run
//! `cargo test -p tag --test tag_crud_contract` and
//! `cargo test -p tag --test tag_service_integration` for the service APIs
//! these pages wrap. Product-surface needles and Playwright live in
//! `cargo test -p tag --test product_surface` and `examples/tag-ui-e2e`.
//!
//! Workspace example `protected-tag-host` covers catalog plus `ManyToMany`
//! without mounting this UI (inventory `tag` / `/tag`):
//! `cargo run -p protected-tag-host`.
//!
//! ## Where to look next
//!
//! - [`server`] — Higgs wrappers; [`TagServerError`] / [`into_server_error`].
//! - [`pages`] — list / create / detail UI.
//! - [`TagCatalogPicker`] — embed API for other products.
//! - [`TagCreateDialog`] — the create dialog the picker opens; reusable on its own.
//! - [`layout`] — `TagAppLayout` (platform shell via `uf-integrations`).
//! - Sibling crate `tag` — schemas, catalog service, history writers.
//! - Root `SECURITY.md` — session-required server fns.

// `uf_app!` / `orbital_routes_extract` emit undocumented associated items.
#![allow(missing_docs)]

use leptos::prelude::*;
use leptos_router::{
    components::{ParentRoute, Route},
    path, Lazy,
};
use uf_product_macros::uf_app;

pub mod components;
/// Shell layout wrapping routed pages ([`TagAppLayout`]).
pub mod layout;
mod lazy_routes;
pub mod pages;
pub mod server;
#[cfg(all(test, feature = "ssr"))]
mod test_support;

pub use components::{TagCatalogPicker, TagCreateDialog};
#[cfg(feature = "preview")]
pub use components::{TAGCATALOGPICKER_DOC, TAGCATALOGPICKER_PROPS};

#[cfg(feature = "preview")]
pub mod preview;
pub use layout::TagAppLayout;
pub use lazy_routes::{
    prefetch_family, TagCreateRoute, TagDetailRoute, TagListRoute, TagVerifiedGuardRouteView,
};
pub use pages::{TagCreatePage, TagDetailPage, TagListPage};
pub use server::{into_server_error, TagServerError};

uf_app! {
    name: "Tags",
    id: "tag",
    description: "Shared tag catalog",
    icon: "🏷️",
    version: "0.1.0",
    routes: TagRoutes,
    route_path: "/tag",
    repository: "https://github.com/unified-field-dev/tag",
}

/// Route tree for the Tags app: list, create, and detail/edit pages, nested under
/// an authenticated+verified guard.
#[allow(missing_docs)]
#[orbital_macros::orbital_routes_extract]
#[component(transparent)]
pub fn TagRoutes() -> impl leptos_router::MatchNestedRoutes + Clone {
    view! {
        <ParentRoute path=path!("tag") view=TagVerifiedGuardRouteView>
            <Route path=path!("") view={Lazy::<TagListRoute>::new()} />
            <Route path=path!("create") view={Lazy::<TagCreateRoute>::new()} />
            <Route path=path!(":id") view={Lazy::<TagDetailRoute>::new()} />
        </ParentRoute>
    }
    .into_inner()
}
