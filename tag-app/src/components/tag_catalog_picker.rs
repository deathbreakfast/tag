//! Multi-select tag catalog picker (Orbital `TagPicker` + tag search source).

use std::collections::HashMap;

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::components::A;
use orbital_macros::component_doc;
use tag::types::TagDetailDto;
use tag::{normalize_name_key, TagSearchSourceId};
use uf_product::components::{
    Caption1, Flex, FlexGap, Tag, TagPicker, TagPickerBind, TagPickerControl, TagPickerGroup,
    TagPickerInput, TagPickerOption, TagPickerSize,
};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use super::TagCreateDialog;
use crate::server::search_tag_catalog;

const CATALOG_LIMIT: u32 = 50;
const CREATE_OPTION_VALUE: &str = "__create";

/// Multi-select shared tag catalog (Orbital `TagPicker` + Tag's registered
/// search source).
///
/// # When to use
///
/// - Connection or association UIs that pick tags from the shared catalog
/// - Forms where selected tag ids are stored and synced via `on_change`
/// - Filters over tagged records, with `allow_create=false`
///
/// For custom tag chips or a generic multi-select without the shared catalog, use
/// **Data Display** (`Tag`, `Tag Group`) and **Inputs** (`Tag Picker`) previews.
///
/// # Usage
///
/// Bind `selected` to the current tag id list and wire `on_change` to your
/// connection APIs. Typing in the chip input searches the catalog through
/// [`search_tag_catalog`](crate::server::search_tag_catalog). Pass `seed` with
/// the already-attached tags so their chips show names before the first search
/// returns.
///
/// When nothing selectable matches the typed text and no tag has that name, the
/// list offers `<text> (new)`. Choosing it opens [`TagCreateDialog`]; the created
/// tag is selected automatically. Set `allow_create=false` in filters, where
/// creating a tag makes no sense.
#[component_doc(
    category = "Unified Field",
    preview_slug = "tag-catalog-picker",
    preview_label = "Tag Catalog Picker",
    preview_icon = icondata::AiTagOutlined,
    preview = "manual",
)]
#[component]
pub fn TagCatalogPicker(
    /// Selected tag ids (two-way).
    selected: RwSignal<Vec<String>>,
    /// Called with the full id list whenever the selection changes.
    on_change: Callback<Vec<String>>,
    /// Link target for "Manage tags" (defaults to `/tag`).
    #[prop(optional, into)]
    manage_tags_href: Option<String>,
    /// Known tags (for example the ones already attached) used to label chips
    /// before any search result arrives.
    #[prop(optional, into)]
    seed: Signal<Vec<SearchSourceItem>>,
    /// Offer `<text> (new)` to create a missing tag (default `true`).
    #[prop(default = true)]
    allow_create: bool,
) -> impl IntoView {
    let manage_href = manage_tags_href.unwrap_or_else(|| "/tag".to_string());
    let query = RwSignal::new(String::new());
    let create_open = RwSignal::new(false);
    let create_name = RwSignal::new(String::new());
    let input_generation = RwSignal::new(0u32);
    let CatalogSearch {
        results,
        results_query,
        labels,
        error,
    } = CatalogSearch::watch(query, seed);

    Effect::new(move |_| {
        on_change.run(selected.get());
    });

    let create_offer = Memo::new(move |_| {
        let q = query.get();
        if results_query.get().as_deref() != Some(q.as_str()) {
            return None;
        }
        create_offer_text(&q, &results.read(), &selected.read(), allow_create)
    });

    let on_created = Callback::new(move |tag: TagDetailDto| {
        labels.update(|cache| {
            cache.insert(tag.id.clone(), tag.name.clone());
        });
        selected.update(|ids| {
            if !ids.contains(&tag.id) {
                ids.push(tag.id);
            }
        });
        query.set(String::new());
        input_generation.update(|n| *n = n.wrapping_add(1));
    });

    view! {
        <div data-testid="tag-catalog-picker">
            <Flex vertical=true gap=FlexGap::Small>
            <TagPicker bind=TagPickerBind::new(selected) size=Signal::from(TagPickerSize::Medium)>
                <TagPickerControl slot>
                    <TagPickerGroup>
                        <For
                            each=move || selected.get()
                            key=|id| id.clone()
                            let:id
                        >
                            <Tag value=id.clone()>
                                {move || label_for(&labels.read(), &id)}
                            </Tag>
                        </For>
                    </TagPickerGroup>
                    // Re-mounting the input is how the typed text clears after an inline create.
                    {move || {
                        input_generation.track();
                        view! {
                            <TagPickerInput on_search=Callback::new(move |q: String| query.set(q)) />
                        }
                    }}
                </TagPickerControl>
                <For
                    each=move || results.get()
                    key=|item| item.id.clone()
                    let:item
                >
                    <TagPickerOption value=item.id.clone() text=item.title />
                </For>
                {move || create_offer.get().map(|text| view! {
                    <div data-testid="tag-catalog-picker-create-option">
                        <TagPickerOption
                            value=CREATE_OPTION_VALUE.to_string()
                            text=format!("{text} (new)")
                            on_activate=Callback::new(move |()| {
                                create_name.set(text.clone());
                                create_open.set(true);
                            })
                        />
                    </div>
                })}
            </TagPicker>
            <div data-testid="tag-catalog-picker-manage">
                <A href=manage_href>
                    <Caption1>"Manage tags →"</Caption1>
                </A>
            </div>
            {move || error.get().map(|msg| view! {
                <Caption1>{msg}</Caption1>
            })}
            </Flex>
            {allow_create.then(|| view! {
                <TagCreateDialog open=create_open initial_name=create_name on_created />
            })}
        </div>
    }
}

/// Catalog search state driven by the typed query.
#[derive(Clone, Copy)]
struct CatalogSearch {
    /// Rows from the newest search response.
    results: RwSignal<Vec<SearchSourceItem>>,
    /// The query `results` answers; `None` until the first response.
    results_query: RwSignal<Option<String>>,
    /// Id → title for every tag seen, from `seed` or any response.
    labels: RwSignal<HashMap<String, String>>,
    /// Message from the last failed search.
    error: RwSignal<Option<String>>,
}

impl CatalogSearch {
    /// Search the catalog on every `query` change and keep `seed` titles in
    /// the label cache.
    fn watch(query: RwSignal<String>, seed: Signal<Vec<SearchSourceItem>>) -> Self {
        let state = Self {
            results: RwSignal::new(Vec::new()),
            results_query: RwSignal::new(None),
            labels: RwSignal::new(HashMap::new()),
            error: RwSignal::new(None),
        };
        let request_seq = StoredValue::new(RequestSequence::default());
        let source_keys = vec![SearchSourceKey::new(
            TagSearchSourceId::Catalog.as_str(),
            TagSearchSourceId::Catalog.label(),
        )];

        Effect::new(move |_| {
            let seeded = seed.get();
            state.labels.update(|cache| remember_labels(cache, &seeded));
        });

        Effect::new(move |_| {
            let q = query.get();
            let keys = source_keys.clone();
            let mut ticket = 0;
            request_seq.update_value(|seq| ticket = seq.next());
            spawn_local_scoped(async move {
                let arg = (!q.is_empty()).then(|| q.clone());
                let outcome = search_tag_catalog(keys, arg, CATALOG_LIMIT).await;
                if !request_seq.with_value(|seq| seq.is_latest(ticket)) {
                    return;
                }
                match outcome {
                    Ok(rows) => {
                        state.labels.update(|cache| remember_labels(cache, &rows));
                        state.results.set(rows);
                        state.results_query.set(Some(q));
                        state.error.set(None);
                    }
                    Err(err) => state.error.set(Some(err.to_string())),
                }
            });
        });

        state
    }
}

/// Monotonic ticket so only the newest search response is applied.
#[derive(Default)]
struct RequestSequence {
    latest: u32,
}

impl RequestSequence {
    const fn next(&mut self) -> u32 {
        self.latest = self.latest.wrapping_add(1);
        self.latest
    }

    const fn is_latest(&self, ticket: u32) -> bool {
        self.latest == ticket
    }
}

/// Keep id → title for every item seen, so chips stay labeled when a later
/// search no longer returns them.
fn remember_labels(cache: &mut HashMap<String, String>, items: &[SearchSourceItem]) {
    for item in items {
        cache.insert(item.id.clone(), item.title.clone());
    }
}

fn label_for(cache: &HashMap<String, String>, id: &str) -> String {
    cache.get(id).cloned().unwrap_or_else(|| id.to_string())
}

/// The trimmed text to offer as `<text> (new)`, if the picker should offer it.
///
/// Offered only when creating is allowed, the text isn't blank, no unselected
/// result is left to pick, and no result already has that name.
fn create_offer_text(
    query: &str,
    results: &[SearchSourceItem],
    selected: &[String],
    allow_create: bool,
) -> Option<String> {
    let text = query.trim();
    if !allow_create || text.is_empty() {
        return None;
    }
    let key = normalize_name_key(text);
    if results
        .iter()
        .any(|item| normalize_name_key(&item.title) == key)
    {
        return None;
    }
    if results.iter().any(|item| !selected.contains(&item.id)) {
        return None;
    }
    Some(text.to_string())
}

#[cfg(test)]
mod tests {
    use super::{create_offer_text, label_for, remember_labels, RequestSequence};
    use std::collections::HashMap;
    use uf_search_core::SearchSourceItem;

    fn item(id: &str, title: &str) -> SearchSourceItem {
        SearchSourceItem {
            source_id: "tag_catalog_search_source".into(),
            id: id.into(),
            title: title.into(),
            description: None,
            kind: "tag".into(),
        }
    }

    #[test]
    fn label_cache_keeps_titles_across_searches_happy() {
        let mut cache = HashMap::new();
        remember_labels(&mut cache, &[item("1", "Finance"), item("2", "Travel")]);
        remember_labels(&mut cache, &[item("3", "Ops")]);
        assert_eq!(label_for(&cache, "1"), "Finance");
        assert_eq!(label_for(&cache, "3"), "Ops");
    }

    #[test]
    fn seed_supplies_label_before_fetch_happy() {
        let mut cache = HashMap::new();
        remember_labels(&mut cache, &[item("7", "Seeded")]);
        assert_eq!(label_for(&cache, "7"), "Seeded");
    }

    #[test]
    fn unknown_id_falls_back_to_id_sad() {
        assert_eq!(label_for(&HashMap::new(), "abc"), "abc");
    }

    #[test]
    fn latest_request_sequence_wins() {
        let mut seq = RequestSequence::default();
        let first = seq.next();
        let second = seq.next();
        assert!(!seq.is_latest(first));
        assert!(seq.is_latest(second));
    }

    #[test]
    fn offer_create_when_no_results_happy() {
        assert_eq!(
            create_offer_text(" Groceries ", &[], &[], true).as_deref(),
            Some("Groceries")
        );
    }

    #[test]
    fn offer_create_when_all_results_selected_happy() {
        let results = [item("1", "Grocery run")];
        assert_eq!(
            create_offer_text("Groc", &results, &["1".into()], true).as_deref(),
            Some("Groc")
        );
        assert!(create_offer_text("Groc", &results, &[], true).is_none());
    }

    #[test]
    fn no_create_for_blank_query_sad() {
        assert!(create_offer_text("   ", &[], &[], true).is_none());
    }

    #[test]
    fn no_create_when_same_name_exists_sad() {
        let results = [item("1", "Ops")];
        for typed in ["ops", " OPS ", "Ops"] {
            assert!(
                create_offer_text(typed, &results, &[], true).is_none(),
                "{typed:?}"
            );
            assert!(
                create_offer_text(typed, &results, &["1".into()], true).is_none(),
                "{typed:?} selected"
            );
        }
    }

    #[test]
    fn no_create_when_allow_create_false_sad() {
        assert!(create_offer_text("Groceries", &[], &[], false).is_none());
    }
}
