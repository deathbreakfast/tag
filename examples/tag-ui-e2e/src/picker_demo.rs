//! `/dev/tag-picker`: one linked `TagCatalogPicker` for the picker specs.
//!
//! `?seed=` takes a JSON list of `{"id", "name"}` tags that start selected.
//! The readout under the picker shows the ids from the last `on_change`.

use leptos::prelude::*;
use leptos_router::hooks::use_query_map;
use serde::Deserialize;
use tag_app::TagCatalogPicker;
use uf_integrations::{
    HostAuthMenu, ShellAppBar, ShellAuthMenu, UnifiedFieldAppBar, UnifiedFieldShellLayout,
};
use uf_product::components::{Caption1, ContentContainer};
use uf_product::primitives::{Flex, FlexGap, Title3};
use uf_search_core::SearchSourceItem;

#[derive(Deserialize)]
struct SeedTag {
    id: String,
    name: String,
}

fn seed_items(raw: Option<String>) -> Vec<SearchSourceItem> {
    let tags: Vec<SeedTag> = raw
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();
    tags.into_iter()
        .map(|tag| SearchSourceItem {
            source_id: "tag_catalog_search_source".into(),
            id: tag.id,
            title: tag.name,
            description: None,
            kind: "tag".into(),
            image_url: None,
        })
        .collect()
}

#[component]
pub fn TagPickerDemoPage() -> impl IntoView {
    // Read once: the picker owns the selection after mount.
    let seed = seed_items(use_query_map().with_untracked(|q| q.get("seed")));
    let selected = RwSignal::new(seed.iter().map(|item| item.id.clone()).collect::<Vec<_>>());
    let last_change = RwSignal::new(None::<Vec<String>>);
    let on_change = Callback::new(move |ids: Vec<String>| last_change.set(Some(ids)));

    view! {
        <UnifiedFieldShellLayout>
            <ShellAppBar slot>
                <UnifiedFieldAppBar app_name="Tag e2e".to_string()>
                    <ShellAuthMenu slot:auth_menu>
                        <HostAuthMenu />
                    </ShellAuthMenu>
                </UnifiedFieldAppBar>
            </ShellAppBar>
            <ContentContainer max_width="640px" data_testid="tag-picker-demo">
                <Flex vertical=true gap=FlexGap::Medium>
                    <Title3>"Tag picker"</Title3>
                    <TagCatalogPicker
                        selected
                        on_change
                        seed=Signal::stored(seed)
                        link_chips=true
                    />
                    <div data-testid="tag-picker-demo-change">
                        <Caption1>
                            {move || last_change.get().map(|ids| format!("{ids:?}"))}
                        </Caption1>
                    </div>
                </Flex>
            </ContentContainer>
        </UnifiedFieldShellLayout>
    }
}
