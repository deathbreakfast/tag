//! Multi-select tag catalog picker (Orbital `ChipSelector` + tag search source).

use std::collections::HashMap;

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::components::A;
use orbital_macros::component_doc;
use tag::types::TagDetailDto;
use tag::{normalize_name_key, TagSearchSourceId};
use uf_product::components::{Caption1, ChipOption, ChipSelector, Flex, FlexGap, TagPickerSize};
use uf_search_core::{SearchSourceItem, SearchSourceKey};

use super::TagCreateDialog;
use crate::server::search_tag_catalog;

const CATALOG_LIMIT: u32 = 50;
const CREATE_OPTION_VALUE: &str = "__create";

/// Multi-select shared tag catalog (Orbital `ChipSelector` + Tag's registered
/// search source).
///
/// # When to use
///
/// - Connection or association UIs that pick tags from the shared catalog
/// - Forms where selected tag ids are stored and synced via `on_change`
/// - Filters over tagged records, with `allow_create=false`
///
/// For a generic chip field without the shared catalog, use **Inputs**
/// (`Chip Selector`) in the Orbital previews.
///
/// # Usage
///
/// Bind `selected` to the current tag id list and wire `on_change` to your
/// connection APIs. Selected tags show as outlined chips inside the input, each
/// with its own remove button; the field wraps onto new rows as chips are
/// added. Typing searches the catalog through
/// [`search_tag_catalog`](crate::server::search_tag_catalog). Pass `seed` with
/// the already-attached tags so their chips show names before the first search
/// returns. Removing the last chip calls `on_change` with an empty list.
///
/// `size` defaults to `Large`, the size Orbital's other chip fields (such as
/// Magnon's Projects) use. Set `link_chips` to make each chip's name a link to
/// its tag page (`{manage_tags_href}/{id}`, so `/tag/{id}` by default); the
/// chip's remove button stays a separate control.
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
    /// Link target for "Manage tags" (defaults to `/tag`). Linked chips point
    /// at `{manage_tags_href}/{id}`.
    #[prop(optional, into)]
    manage_tags_href: Option<String>,
    /// Known tags (for example the ones already attached) used to label chips
    /// before any search result arrives.
    #[prop(optional, into)]
    seed: Signal<Vec<SearchSourceItem>>,
    /// Offer `<text> (new)` to create a missing tag (default `true`).
    #[prop(default = true)]
    allow_create: bool,
    /// Show the "Manage tags" link under the picker (default `true`). Hosts
    /// that line the picker up with other controls in a row turn it off and
    /// place their own link beside the field.
    #[prop(default = true)]
    manage_link: bool,
    /// Accessible name for the search input (default "Tags"). Set it when the
    /// picker chooses something narrower, such as "Graph colour tags".
    #[prop(optional, into)]
    aria_label: Option<String>,
    /// Control size (default `Large`, as on Orbital's other chip fields).
    #[prop(default = TagPickerSize::Large.into(), into)]
    size: Signal<TagPickerSize>,
    /// Link each chip's name to its tag page (default `false`).
    #[prop(default = false)]
    link_chips: bool,
) -> impl IntoView {
    let manage_href = manage_tags_href.unwrap_or_else(|| "/tag".to_string());
    let chip_href_base = link_chips.then(|| manage_href.trim_end_matches('/').to_string());
    let aria_label = aria_label.unwrap_or_else(|| "Tags".to_string());
    let query = RwSignal::new(String::new());
    let create_open = RwSignal::new(false);
    let create_name = RwSignal::new(String::new());
    // Text of the last `(new)` row offered. Kept after the pick clears the
    // query, so the dialog opens with what the user typed.
    let offered_name = StoredValue::new(String::new());
    let CatalogSearch {
        results,
        results_query,
        labels,
        error,
    } = CatalogSearch::watch(query, seed);

    Effect::new(move |_| {
        let ids = selected.get();
        // Picking `(new)` lands the placeholder key here for a moment; the
        // effect below takes it out again, and that change reports the list.
        if !ids.iter().any(|id| id == CREATE_OPTION_VALUE) {
            on_change.run(ids);
        }
    });

    Effect::new(move |_| {
        if selected.with(|ids| ids.iter().any(|id| id == CREATE_OPTION_VALUE)) {
            selected.update(|ids| ids.retain(|id| id != CREATE_OPTION_VALUE));
            create_name.set(offered_name.get_value());
            create_open.set(true);
        }
    });

    let create_offer = Memo::new(move |_| {
        let q = query.get();
        if results_query.get().as_deref() != Some(q.as_str()) {
            return None;
        }
        create_offer_text(&q, &results.read(), &selected.read(), allow_create)
    });

    let options = Signal::derive(move || {
        let offer = create_offer.get();
        if let Some(text) = &offer {
            offered_name.set_value(text.clone());
        }
        chip_options(
            &results.read(),
            &selected.read(),
            &labels.read(),
            chip_href_base.as_deref(),
            offer.as_deref(),
        )
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
    });

    let empty_placeholder = if allow_create {
        "No tags. Search or create one"
    } else {
        "No tags. Search tags"
    };

    view! {
        <div data-testid="tag-catalog-picker">
            <Flex vertical=true gap=FlexGap::Small>
            <ChipSelector
                bind=selected
                options
                size
                placeholder="Add a tag"
                empty_placeholder=empty_placeholder
                aria_label=aria_label
                on_search=Callback::new(move |q: String| query.set(q))
            />
            {manage_link.then(|| view! {
                <div data-testid="tag-catalog-picker-manage">
                    <A href=manage_href>
                        <Caption1>"Manage tags →"</Caption1>
                    </A>
                </div>
            })}
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
    fn new() -> Self {
        Self {
            results: RwSignal::new(Vec::new()),
            results_query: RwSignal::new(None),
            labels: RwSignal::new(HashMap::new()),
            error: RwSignal::new(None),
        }
    }

    fn watch(query: RwSignal<String>, seed: Signal<Vec<SearchSourceItem>>) -> Self {
        let state = Self::new();
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
                state.apply_response(request_seq, ticket, q, outcome);
            });
        });

        state
    }

    /// Store one search response, unless a newer search has started since.
    ///
    /// The picker can be unmounted while a search is in flight (a dialog around
    /// it re-renders). Its signals are disposed by then, so a late response is
    /// dropped instead of written.
    fn apply_response<E: std::fmt::Display>(
        self,
        request_seq: StoredValue<RequestSequence>,
        ticket: u32,
        q: String,
        outcome: Result<Vec<SearchSourceItem>, E>,
    ) {
        let latest = request_seq
            .try_with_value(|seq| seq.is_latest(ticket))
            .unwrap_or(false);
        if !latest {
            return;
        }
        match outcome {
            Ok(rows) => {
                if self
                    .labels
                    .try_update(|cache| remember_labels(cache, &rows))
                    .is_none()
                {
                    return;
                }
                let _ = self.results.try_set(rows);
                let _ = self.results_query.try_set(Some(q));
                let _ = self.error.try_set(None);
            }
            Err(err) => {
                let _ = self.error.try_set(Some(err.to_string()));
            }
        }
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

/// Options for the chip selector: the newest search rows, then any selected
/// tag the rows left out (so its chip keeps its name), then the `(new)` row.
fn chip_options(
    results: &[SearchSourceItem],
    selected: &[String],
    labels: &HashMap<String, String>,
    href_base: Option<&str>,
    create_offer: Option<&str>,
) -> Vec<ChipOption> {
    let option = |id: &str, text: String| {
        let option = ChipOption::new(id, text);
        match href_base {
            Some(base) => option.href(format!("{base}/{id}")),
            None => option,
        }
    };
    let mut options: Vec<ChipOption> = results
        .iter()
        .map(|item| option(&item.id, item.title.clone()))
        .collect();
    for id in selected {
        if id != CREATE_OPTION_VALUE && !results.iter().any(|item| &item.id == id) {
            options.push(option(id, label_for(labels, id)));
        }
    }
    if let Some(text) = create_offer {
        options.push(ChipOption::new(
            CREATE_OPTION_VALUE,
            format!("{text} (new)"),
        ));
    }
    options
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
    use super::{
        chip_options, create_offer_text, label_for, remember_labels, CatalogSearch,
        RequestSequence, CREATE_OPTION_VALUE,
    };
    use leptos::prelude::*;
    use std::collections::HashMap;
    use uf_search_core::SearchSourceItem;

    fn item(id: &str, title: &str) -> SearchSourceItem {
        SearchSourceItem {
            source_id: "tag_catalog_search_source".into(),
            id: id.into(),
            title: title.into(),
            description: None,
            kind: "tag".into(),
            image_url: None,
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

    #[test]
    fn chip_options_keep_selected_labels_happy() {
        let mut labels = HashMap::new();
        remember_labels(&mut labels, &[item("7", "Seeded")]);
        let options = chip_options(&[item("1", "Finance")], &["7".into()], &labels, None, None);
        let pairs: Vec<_> = options
            .iter()
            .map(|o| (o.value.as_str(), o.text.as_str()))
            .collect();
        assert_eq!(pairs, [("1", "Finance"), ("7", "Seeded")]);
        assert!(options.iter().all(|o| o.href.is_none()));
    }

    #[test]
    fn chip_options_selected_in_results_not_repeated_happy() {
        let options = chip_options(
            &[item("1", "Finance")],
            &["1".into()],
            &HashMap::new(),
            None,
            None,
        );
        assert_eq!(options.len(), 1);
    }

    #[test]
    fn chip_options_link_to_tag_page_happy() {
        let options = chip_options(
            &[item("1", "Finance")],
            &["9".into()],
            &HashMap::new(),
            Some("/tag"),
            None,
        );
        let hrefs: Vec<_> = options.iter().map(|o| o.href.as_deref()).collect();
        assert_eq!(hrefs, [Some("/tag/1"), Some("/tag/9")]);
    }

    #[test]
    fn chip_options_create_row_last_and_unlinked_happy() {
        let options = chip_options(&[], &[], &HashMap::new(), Some("/tag"), Some("Groceries"));
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].value, CREATE_OPTION_VALUE);
        assert_eq!(options[0].text, "Groceries (new)");
        assert!(options[0].href.is_none());
    }

    #[test]
    fn chip_options_skip_create_key_in_selection_sad() {
        let options = chip_options(
            &[],
            &[CREATE_OPTION_VALUE.into()],
            &HashMap::new(),
            None,
            None,
        );
        assert!(options.is_empty());
    }

    /// A search state and one in-flight ticket, created under `owner`.
    fn in_flight(owner: &Owner) -> (CatalogSearch, StoredValue<RequestSequence>, u32) {
        owner.with(|| {
            let state = CatalogSearch::new();
            let seq = StoredValue::new(RequestSequence::default());
            let mut ticket = 0;
            seq.update_value(|s| ticket = s.next());
            (state, seq, ticket)
        })
    }

    #[test]
    fn search_response_fills_results_happy() {
        let owner = Owner::new();
        let (state, seq, ticket) = in_flight(&owner);
        state.apply_response(seq, ticket, "fin".into(), Ok::<_, String>(vec![item("1", "Finance")]));
        assert_eq!(state.results.get_untracked(), vec![item("1", "Finance")]);
        assert_eq!(state.results_query.get_untracked().as_deref(), Some("fin"));
        assert_eq!(state.labels.with_untracked(|c| label_for(c, "1")), "Finance");
    }

    #[test]
    fn superseded_search_response_is_dropped_sad() {
        let owner = Owner::new();
        let (state, seq, stale) = in_flight(&owner);
        seq.update_value(|s| {
            s.next();
        });
        state.apply_response(seq, stale, "fin".into(), Ok::<_, String>(vec![item("1", "Finance")]));
        assert!(state.results.get_untracked().is_empty());
        assert_eq!(state.results_query.get_untracked(), None);
    }

    #[test]
    fn search_response_after_unmount_is_dropped_sad() {
        let owner = Owner::new();
        let (state, seq, ticket) = in_flight(&owner);
        owner.cleanup();
        // Used to panic reading the disposed request sequence (a wasm trap in the browser).
        state.apply_response(seq, ticket, "fin".into(), Ok::<_, String>(vec![item("1", "Finance")]));
        state.apply_response(seq, ticket, "fin".into(), Err::<Vec<_>, _>("offline"));
        assert!(state.results.try_get_untracked().is_none());
    }
}
