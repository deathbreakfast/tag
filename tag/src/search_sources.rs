//! Tag catalog search source for Orbital pickers.
//!
//! Provider queries go through [`crate::list`]; failures convert into the
//! `anyhow` search-source result via [`crate::TagError`]'s `std::error::Error`
//! impl.
#![allow(missing_docs)]

uf_product_macros::define_search_sources! {
    enum TagSearchSourceId {
        Catalog => {
            id: "tag_catalog_search_source",
            label: "Tags",
            description: "Shared tag catalog",
            provider: TagCatalogSearchSource,
        },
    }
}

/// [`uf_search_core::SearchSourceProvider`] exposing the tag catalog to Orbital search pickers.
#[cfg(feature = "ssr")]
pub struct TagCatalogSearchSource;

#[cfg(feature = "ssr")]
impl uf_search_core::SearchSourceProvider for TagCatalogSearchSource {
    fn query<'a>(
        &'a self,
        v: &'a valence::Valence,
        query_text: &'a str,
        max_results: u32,
    ) -> uf_search_core::SearchSourceFuture<'a> {
        Box::pin(async move {
            let search = if query_text.trim().is_empty() {
                None
            } else {
                Some(query_text.trim().to_string())
            };
            let rows = crate::list(v, search, None).await?;
            Ok(
                catalog_items_from_list(rows, TagSearchSourceId::Catalog.as_str())
                    .into_iter()
                    .take(max_results as usize)
                    .collect(),
            )
        })
    }
}

/// Map [`crate::types::TagRowDto`] rows into generic [`uf_search_core::SearchSourceItem`]s
/// tagged with `source_id`.
#[cfg(feature = "ssr")]
pub fn catalog_items_from_list(
    rows: Vec<crate::types::TagRowDto>,
    source_id: &str,
) -> Vec<uf_search_core::SearchSourceItem> {
    rows.into_iter()
        .map(|row| uf_search_core::SearchSourceItem {
            source_id: source_id.to_string(),
            id: row.id,
            title: row.name,
            description: row.taxonomy,
            kind: "tag".to_string(),
            image_url: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn catalog_source_id_round_trip() {
        assert_eq!(
            super::TagSearchSourceId::from_source_id("tag_catalog_search_source"),
            Some(super::TagSearchSourceId::Catalog)
        );
    }

    #[cfg(feature = "ssr")]
    #[test]
    fn catalog_items_from_list_maps_fields() {
        use chrono::Utc;

        let items = super::catalog_items_from_list(
            vec![crate::types::TagRowDto {
                id: "1".into(),
                name: "N".into(),
                taxonomy: Some("t".into()),
                description: Some("d".into()),
                owner_display: "you".into(),
                updated_at: Utc::now(),
            }],
            "tag_catalog_search_source",
        );
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_id, "tag_catalog_search_source");
        assert_eq!(items[0].id, "1");
        assert_eq!(items[0].title, "N");
        assert_eq!(items[0].description.as_deref(), Some("t"));
        assert_eq!(items[0].kind, "tag");
    }
}
