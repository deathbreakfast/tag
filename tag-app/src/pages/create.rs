//! Create-tag form page; navigates to detail on success.

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use leptos_router::hooks::use_navigate;
use leptos_router::NavigateOptions;
use uf_product::components::{
    Button, ButtonAppearance, Caption1, Card, ContentContainer, Flex, FlexGap, FlexJustify,
    MessageBar, MessageBarIntent, SpacingSize, Title3,
};

use crate::components::{tag_create_input, TagCreateFields};
use crate::server::create_tag;

/// Form page for creating a new tag; navigates to the detail page on success.
#[component]
pub fn TagCreatePage() -> impl IntoView {
    let navigate = use_navigate();
    let name = RwSignal::new(String::new());
    let taxonomy = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let submitting = RwSignal::new(false);

    let on_submit = move |_| {
        let navigate = navigate.clone();
        let payload = tag_create_input(name.get(), taxonomy.get(), description.get());
        error.set(None);
        submitting.set(true);
        spawn_local_scoped(async move {
            match create_tag(payload).await {
                Ok(created) => {
                    submitting.set(false);
                    navigate(&format!("/tag/{}", created.id), NavigateOptions::default());
                }
                Err(err) => {
                    submitting.set(false);
                    error.set(Some(err.to_string()));
                }
            }
        });
    };

    view! {
        <ContentContainer max_width="720px" data_testid="tag-create-page">
            <Flex vertical=true gap=FlexGap::Medium>
                <Card>
                    <Flex vertical=true gap=FlexGap::Small padding=SpacingSize::Size200.inset()>
                        <Title3>"Create tag"</Title3>
                        <Caption1>"Add a shared label to the catalog."</Caption1>
                    </Flex>
                </Card>

                <Card>
                    <Flex vertical=true gap=FlexGap::Medium padding=SpacingSize::Size200.inset()>
                        <TagCreateFields name taxonomy description />

                        {move || error.get().map(|msg| view! {
                            <MessageBar intent=MessageBarIntent::Error>{msg}</MessageBar>
                        })}

                        <Flex justify=FlexJustify::End gap=FlexGap::Small>
                            <Button
                                appearance=ButtonAppearance::Primary
                                on_click=Callback::new(on_submit)
                                disabled=Signal::derive(move || submitting.get())
                                attr:data-testid="tag-create-submit"
                            >
                                "Create"
                            </Button>
                        </Flex>
                    </Flex>
                </Card>
            </Flex>
        </ContentContainer>
    }
}
