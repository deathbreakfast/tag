//! Tag detail/edit page: load by route id, mutate fields, embed audit timeline.

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::components::A;
use leptos_router::hooks::{use_navigate, use_params_map};
use leptos_router::NavigateOptions;
#[cfg(any(feature = "hydrate", feature = "ssr"))]
use record_history_leptos::HistoryTimeline;
use tag::types::{TagDetailDto, TagUpdateInput};
use uf_product::components::{
    Button, ButtonAppearance, Caption1, Card, ContentContainer, Field, Flex, FlexAlign, FlexGap,
    FlexJustify, Input, MessageBar, MessageBarIntent, Skeleton, SkeletonItem, SpacingSize,
    Textarea, Title3,
};
use valence::RecordId;

use crate::server::{delete_tag, get_tag, update_tag};

/// Audit timeline for the tag under edit.
///
/// `HistoryTimeline` is only exported by `record-history-leptos` when a client
/// or server rendering feature is enabled, so featureless check builds get a
/// no-op stub instead.
///
/// Callers remount by recreating this view inside a reactive block keyed on a
/// refresh nonce after Save so the infinite-scroll Resource reloads history.
#[cfg(any(feature = "hydrate", feature = "ssr"))]
fn history_timeline(source: RecordId) -> impl IntoView {
    view! { <HistoryTimeline source=source /> }
}

#[cfg(not(any(feature = "hydrate", feature = "ssr")))]
fn history_timeline(_source: RecordId) -> impl IntoView {}

/// Placeholder while tag detail loads.
#[component]
fn TagDetailSkeleton() -> impl IntoView {
    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <Card>
                <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                    <Skeleton>
                        <SkeletonItem width="30%".to_string() height="20px".to_string() />
                    </Skeleton>
                    <Skeleton>
                        <SkeletonItem width="50%".to_string() height="32px".to_string() />
                    </Skeleton>
                </Flex>
            </Card>
            <Card>
                <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                    <Skeleton>
                        <SkeletonItem width="100%".to_string() height="40px".to_string() />
                    </Skeleton>
                    <Skeleton>
                        <SkeletonItem width="100%".to_string() height="40px".to_string() />
                    </Skeleton>
                    <Skeleton>
                        <SkeletonItem width="100%".to_string() height="120px".to_string() />
                    </Skeleton>
                </Flex>
            </Card>
        </Flex>
    }
}

#[component]
fn TagDetailEditor(row: TagDetailDto, tag_id: String, refresh: RwSignal<u32>) -> impl IntoView {
    let navigate = use_navigate();
    let name = RwSignal::new(row.name.clone());
    let taxonomy = RwSignal::new(row.taxonomy.clone().unwrap_or_default());
    let description = RwSignal::new(row.description.clone().unwrap_or_default());
    let error = RwSignal::new(None::<String>);
    let saving = RwSignal::new(false);
    let deleting = RwSignal::new(false);
    let history_tag_id = tag_id.clone();

    let save_id = tag_id.clone();
    let on_save = move |_| {
        let payload = TagUpdateInput {
            name: Some(name.get()),
            taxonomy: {
                let t = taxonomy.get();
                if t.trim().is_empty() {
                    Some(String::new())
                } else {
                    Some(t)
                }
            },
            description: {
                let d = description.get();
                if d.trim().is_empty() {
                    Some(String::new())
                } else {
                    Some(d)
                }
            },
        };
        error.set(None);
        saving.set(true);
        let id = save_id.clone();
        spawn_local_scoped(async move {
            match update_tag(id, payload).await {
                Ok(_) => {
                    saving.set(false);
                    refresh.update(|n| *n += 1);
                }
                Err(err) => {
                    saving.set(false);
                    error.set(Some(err.to_string()));
                }
            }
        });
    };

    let delete_id = tag_id;
    let on_delete = move |_| {
        let id = delete_id.clone();
        let navigate = navigate.clone();
        error.set(None);
        deleting.set(true);
        spawn_local_scoped(async move {
            match delete_tag(id).await {
                Ok(()) => navigate("/tag", NavigateOptions::default()),
                Err(err) => {
                    deleting.set(false);
                    error.set(Some(err.to_string()));
                }
            }
        });
    };

    view! {
        <Flex vertical=true gap=FlexGap::Medium>
            <Card>
                <Flex
                    justify=FlexJustify::SpaceBetween
                    align=FlexAlign::Center
                    gap=FlexGap::Medium
                    padding=SpacingSize::Size200.inset()
                >
                    <Flex vertical=true gap=FlexGap::Small>
                        <A href="/tag" attr:data-testid="tag-detail-back">
                            <Caption1>"← Tags"</Caption1>
                        </A>
                        <div data-testid="tag-detail-title">
                            <Title3>{row.name.clone()}</Title3>
                        </div>
                    </Flex>
                    <Flex gap=FlexGap::Small>
                        <Button
                            appearance=ButtonAppearance::Primary
                            on_click=Callback::new(on_save)
                            disabled=Signal::derive(move || saving.get())
                            attr:data-testid="tag-detail-save"
                        >
                            "Save"
                        </Button>
                        <Button
                            appearance=ButtonAppearance::Secondary
                            on_click=Callback::new(on_delete)
                            disabled=Signal::derive(move || deleting.get())
                            attr:data-testid="tag-detail-delete"
                        >
                            "Delete"
                        </Button>
                    </Flex>
                </Flex>
            </Card>

            <Card>
                <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                    <Field label="Name">
                        <div data-testid="tag-detail-name">
                            <Input bind=name />
                        </div>
                    </Field>
                    <Field label="Taxonomy">
                        <div data-testid="tag-detail-taxonomy">
                            <Input bind=taxonomy />
                        </div>
                    </Field>
                    <Field label="Description">
                        <div data-testid="tag-detail-description">
                            <Textarea bind=description />
                        </div>
                    </Field>
                    {move || error.get().map(|msg| view! {
                        <MessageBar intent=MessageBarIntent::Error>{msg}</MessageBar>
                    })}
                </Flex>
            </Card>

            <Card>
                <Flex padding=SpacingSize::Size200.inset()>
                    {move || {
                        // Track refresh so Save remounts the timeline view.
                        let _ = refresh.get();
                        history_timeline(RecordId::new("tag", history_tag_id.clone()))
                    }}
                </Flex>
            </Card>
        </Flex>
    }
}

/// Tag detail/edit page: loads the tag by route `id`, then renders the editor
/// (name/taxonomy/description, save/delete) alongside its audit [`HistoryTimeline`].
#[component]
pub fn TagDetailPage() -> impl IntoView {
    let params = use_params_map();
    let tag_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let refresh = RwSignal::new(0u32);

    let detail = Resource::new(
        move || (tag_id.get(), refresh.get()),
        |(id, _)| async move {
            if id.trim().is_empty() {
                return Ok(None);
            }
            get_tag(id).await
        },
    );

    view! {
        <ContentContainer max_width="900px" data_testid="tag-detail-page">
            <Transition fallback=move || view! { <TagDetailSkeleton /> }>
                {move || {
                    let id = tag_id.get();
                    if id.trim().is_empty() {
                        return view! {
                            <MessageBar intent=MessageBarIntent::Warning>"Missing tag id in route."</MessageBar>
                        }
                        .into_any();
                    }
                    match detail.get() {
                        None => view! { <TagDetailSkeleton /> }.into_any(),
                        Some(Err(err)) => view! {
                            <MessageBar intent=MessageBarIntent::Error>{err.to_string()}</MessageBar>
                        }.into_any(),
                        Some(Ok(None)) => view! {
                            <MessageBar intent=MessageBarIntent::Warning>"Tag not found."</MessageBar>
                        }.into_any(),
                        Some(Ok(Some(row))) => {
                            view! {
                                <TagDetailEditor row=row tag_id=id refresh=refresh />
                            }.into_any()
                        }
                    }
                }}
            </Transition>
        </ContentContainer>
    }
}
