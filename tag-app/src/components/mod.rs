//! Reusable UI components for the Tags app (catalog picker, create dialog,
//! list filters/table).
//!
//! Hosts should prefer [`TagCatalogPicker`]; [`TagCreateDialog`] and
//! [`TagCreateFields`] are exported for hosts that need the same create form.
//! List filter/table helpers are `pub(crate)` for the catalog pages.

mod tag_catalog_picker;
mod tag_create_dialog;
mod tag_list_filters;
mod tag_list_results;
mod tag_list_table;

pub use tag_catalog_picker::TagCatalogPicker;
#[cfg(feature = "preview")]
pub use tag_catalog_picker::{TAGCATALOGPICKER_DOC, TAGCATALOGPICKER_PROPS};
pub use tag_create_dialog::{tag_create_input, TagCreateDialog, TagCreateFields};
pub(crate) use tag_list_filters::TagListFilters;
pub(crate) use tag_list_results::TagListResults;
pub(crate) use tag_list_table::TagListTable;
