//! Catalog search for tag picker (search-source pattern).

use leptos::prelude::*;
use uf_search_core::{SearchSourceItem, SearchSourceKey};

#[cfg(feature = "ssr")]
use super::{into_server_error, require_session, valence_from_ctx, TagServerError};

/// Upper bound for picker catalog search results (resource abuse guard).
#[cfg(feature = "ssr")]
const SEARCH_TAG_CATALOG_MAX: u32 = 50;

/// Search the tag catalog for
/// [`TagCatalogPicker`](crate::components::TagCatalogPicker).
///
/// The query runs through the registered search sources
/// (`SearchSourceRegistry::query_many`), so results come from Tag's
/// [`tag::search_sources::TagCatalogSearchSource`] under the session actor.
/// `source_keys` must be empty or only [`tag::TagSearchSourceId::Catalog`]; other
/// source ids return an invalid-source [`ServerFnError`].
#[uf_product_macros::server]
pub async fn search_tag_catalog(
    /// Search sources requested by the picker (catalog only for this fn).
    source_keys: Vec<SearchSourceKey>,
    /// Optional free-text query matched against tag names.
    query: Option<String>,
    /// Maximum number of results requested by the picker (clamped to 1..=50).
    limit: u32,
) -> Result<Vec<SearchSourceItem>, ServerFnError> {
    let keys = catalog_keys(source_keys).map_err(|e| into_server_error("search_tag_catalog", e))?;
    let ctx = higgs::Higgs::from_request().await?;
    require_session(&ctx).map_err(|e| into_server_error("search_tag_catalog", e))?;
    let v = valence_from_ctx(&ctx).map_err(|e| into_server_error("search_tag_catalog", e))?;
    search_catalog(&v, &keys, query.as_deref().unwrap_or_default(), limit)
        .await
        .map_err(|e| into_server_error("search_tag_catalog", e))
}

/// Only the catalog source is served here; no keys means the catalog.
#[cfg(feature = "ssr")]
fn catalog_keys(source_keys: Vec<SearchSourceKey>) -> Result<Vec<SearchSourceKey>, TagServerError> {
    let catalog = tag::TagSearchSourceId::Catalog;
    if let Some(foreign) = source_keys.iter().find(|key| key.id != catalog.as_str()) {
        return Err(TagServerError::InvalidSource(foreign.id.clone()));
    }
    if source_keys.is_empty() {
        return Ok(vec![SearchSourceKey::new(
            catalog.as_str(),
            catalog.label(),
        )]);
    }
    Ok(source_keys)
}

#[cfg(feature = "ssr")]
async fn search_catalog(
    v: &valence::Valence,
    keys: &[SearchSourceKey],
    query: &str,
    limit: u32,
) -> Result<Vec<SearchSourceItem>, TagServerError> {
    uf_search_core::SearchSourceRegistry::auto_discover()
        .query_many(
            keys,
            v,
            query.trim(),
            limit.clamp(1, SEARCH_TAG_CATALOG_MAX),
        )
        .await
        .map_err(|e| TagServerError::SearchSource(e.to_string()))
}

#[cfg(all(test, feature = "ssr"))]
mod tests {
    use super::{catalog_keys, search_catalog, SEARCH_TAG_CATALOG_MAX};
    use crate::server::{require_user, TagServerError};
    use tag::types::TagCreateInput;
    use uf_search_core::SearchSourceKey;

    #[test]
    fn search_tag_catalog_rejects_foreign_source_sad() {
        let err = catalog_keys(vec![SearchSourceKey::new("apps", "Apps")])
            .expect_err("foreign key rejected");
        assert!(matches!(err, TagServerError::InvalidSource(ref id) if id == "apps"));
        let defaulted = catalog_keys(Vec::new()).expect("empty keys default to catalog");
        assert_eq!(defaulted[0].id, tag::TagSearchSourceId::Catalog.as_str());
    }

    #[test]
    fn search_tag_catalog_unauthenticated_sad() {
        assert!(matches!(
            require_user(None),
            Err(TagServerError::NotAuthenticated)
        ));
        assert!(require_user(Some("user-1")).is_ok());
    }

    #[tokio::test]
    async fn search_tag_catalog_dispatches_via_registry_happy() {
        let v = crate::test_support::valence_as_user().await;
        for i in 0..60 {
            tag::create(
                TagCreateInput {
                    name: format!("Budget {i:02}"),
                    taxonomy: None,
                    description: None,
                },
                &v,
            )
            .await
            .expect("seed tag");
        }
        let keys = catalog_keys(Vec::new()).expect("keys");

        let hits = search_catalog(&v, &keys, " Budget 07 ", 10)
            .await
            .expect("search");
        assert_eq!(
            hits.iter().map(|h| h.title.as_str()).collect::<Vec<_>>(),
            ["Budget 07"]
        );
        assert_eq!(hits[0].source_id, tag::TagSearchSourceId::Catalog.as_str());

        let capped = search_catalog(&v, &keys, "", 500).await.expect("search");
        assert_eq!(capped.len(), SEARCH_TAG_CATALOG_MAX as usize);
    }
}
