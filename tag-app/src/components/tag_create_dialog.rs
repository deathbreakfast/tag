//! Tag create form fields and the create dialog used by [`TagCatalogPicker`](super::TagCatalogPicker).

use leptos::prelude::*;
use leptos::task::spawn_local_scoped;
use tag::types::{TagCreateInput, TagDetailDto};
use uf_product::components::{
    Button, ButtonAppearance, Dialog, DialogActions, DialogBody, DialogContent, DialogSurface,
    DialogTitle, Field, Flex, FlexGap, Input, InputAppearance, MessageBar, MessageBarIntent,
    Textarea, TextareaAppearance,
};

use crate::server::{create_tag, find_tag_by_name, is_duplicate_name};

/// Name, Taxonomy and Description fields for creating a tag.
///
/// Shared by the `/tag/create` page and [`TagCreateDialog`]; build the request
/// with [`tag_create_input`].
#[component]
pub fn TagCreateFields(
    /// Tag name (required).
    name: RwSignal<String>,
    /// Optional taxonomy; blank means none.
    taxonomy: RwSignal<String>,
    /// Optional description; blank means none.
    description: RwSignal<String>,
) -> impl IntoView {
    view! {
        <Field label="Name" required=true>
            <div data-testid="tag-create-name">
                <Input bind=name appearance=InputAppearance::with_placeholder("Office Supplies") />
            </div>
        </Field>
        <Field label="Taxonomy">
            <div data-testid="tag-create-taxonomy">
                <Input bind=taxonomy appearance=InputAppearance::with_placeholder("spend") />
            </div>
        </Field>
        <Field label="Description">
            <div data-testid="tag-create-description">
                <Textarea
                    bind=description
                    appearance=TextareaAppearance::with_placeholder("Optional description")
                />
            </div>
        </Field>
    }
}

/// Build a [`TagCreateInput`] from [`TagCreateFields`] values, treating blank
/// taxonomy and description as absent.
#[must_use]
pub fn tag_create_input(name: String, taxonomy: String, description: String) -> TagCreateInput {
    let optional = |s: String| if s.trim().is_empty() { None } else { Some(s) };
    TagCreateInput {
        name,
        taxonomy: optional(taxonomy),
        description: optional(description),
    }
}

/// Dialog that creates a catalog tag, prefilled with a name.
///
/// Opening it resets the form to `initial_name`. Create calls
/// [`create_tag`](crate::server::create_tag); on success `on_created` gets the
/// new tag and the dialog closes. If the name was taken in the meantime, the
/// dialog offers "Use existing tag", which passes the existing tag to
/// `on_created` instead. Other failures stay in the dialog with the typed
/// values intact.
#[component]
pub fn TagCreateDialog(
    /// Whether the dialog is shown.
    open: RwSignal<bool>,
    /// Name to prefill each time the dialog opens.
    #[prop(into)]
    initial_name: Signal<String>,
    /// Receives the created (or existing) tag.
    on_created: Callback<TagDetailDto>,
) -> impl IntoView {
    let name = RwSignal::new(String::new());
    let taxonomy = RwSignal::new(String::new());
    let description = RwSignal::new(String::new());
    let error = RwSignal::new(None::<String>);
    let duplicate_of = RwSignal::new(None::<String>);
    let submitting = RwSignal::new(false);

    Effect::new(move |_| {
        if open.get() {
            name.set(initial_name.get_untracked());
            taxonomy.set(String::new());
            description.set(String::new());
            error.set(None);
            duplicate_of.set(None);
            submitting.set(false);
        }
    });

    let finish = move |tag: TagDetailDto| {
        submitting.set(false);
        open.set(false);
        on_created.run(tag);
    };

    let on_create = move |_| {
        if submitting.get_untracked() {
            return;
        }
        let typed = name.get_untracked();
        let input = tag_create_input(
            typed.clone(),
            taxonomy.get_untracked(),
            description.get_untracked(),
        );
        error.set(None);
        duplicate_of.set(None);
        submitting.set(true);
        spawn_local_scoped(async move {
            match create_tag(input).await {
                Ok(created) => finish(created),
                Err(err) => {
                    submitting.set(false);
                    if is_duplicate_name(&err) {
                        duplicate_of.set(Some(typed.trim().to_string()));
                    } else {
                        error.set(Some(err.to_string()));
                    }
                }
            }
        });
    };

    let on_use_existing = move |_| {
        let Some(typed) = duplicate_of.get_untracked() else {
            return;
        };
        submitting.set(true);
        spawn_local_scoped(async move {
            match find_tag_by_name(typed).await {
                Ok(Some(existing)) => finish(existing),
                Ok(None) => {
                    submitting.set(false);
                    duplicate_of.set(None);
                    error.set(Some("That tag is no longer available.".to_string()));
                }
                Err(err) => {
                    submitting.set(false);
                    error.set(Some(err.to_string()));
                }
            }
        });
    };

    let create_disabled = Signal::derive(move || submitting.get() || name.read().trim().is_empty());

    view! {
        <Dialog open=open>
            <DialogSurface>
                <DialogBody>
                    <DialogTitle>"Create tag"</DialogTitle>
                    <DialogContent>
                        <div data-testid="tag-create-dialog">
                            <Flex vertical=true gap=FlexGap::Medium>
                                <TagCreateFields name taxonomy description />
                                {move || duplicate_of.get().map(|typed| view! {
                                    <DuplicateNameNotice
                                        typed
                                        busy=Signal::derive(move || submitting.get())
                                        on_use_existing=Callback::new(on_use_existing)
                                    />
                                })}
                                {move || error.get().map(|msg| view! {
                                    <MessageBar intent=MessageBarIntent::Error>{msg}</MessageBar>
                                })}
                            </Flex>
                        </div>
                    </DialogContent>
                    <CreateDialogActions open create_disabled on_create=Callback::new(on_create) />
                </DialogBody>
            </DialogSurface>
        </Dialog>
    }
}

/// Warning shown when the typed name already belongs to a tag.
#[component]
fn DuplicateNameNotice(
    typed: String,
    busy: Signal<bool>,
    on_use_existing: Callback<leptos::ev::MouseEvent>,
) -> impl IntoView {
    view! {
        <MessageBar intent=MessageBarIntent::Warning>
            <Flex vertical=true gap=FlexGap::Small>
                {format!("A tag named {typed} already exists.")}
                <Button
                    appearance=ButtonAppearance::Secondary
                    disabled=busy
                    on_click=on_use_existing
                    attr:data-testid="tag-create-dialog-use-existing"
                >
                    "Use existing tag"
                </Button>
            </Flex>
        </MessageBar>
    }
}

/// Cancel and Create buttons for [`TagCreateDialog`].
#[component]
fn CreateDialogActions(
    open: RwSignal<bool>,
    create_disabled: Signal<bool>,
    on_create: Callback<leptos::ev::MouseEvent>,
) -> impl IntoView {
    view! {
        <DialogActions>
            <Button
                appearance=ButtonAppearance::Secondary
                on_click=Callback::new(move |_| open.set(false))
                attr:data-testid="tag-create-dialog-cancel"
            >
                "Cancel"
            </Button>
            <Button
                appearance=ButtonAppearance::Primary
                disabled=create_disabled
                on_click=on_create
                attr:data-testid="tag-create-dialog-submit"
            >
                "Create"
            </Button>
        </DialogActions>
    }
}

#[cfg(test)]
mod tests {
    use super::tag_create_input;

    #[test]
    fn blank_optional_fields_become_none() {
        let input = tag_create_input("Ops".into(), "  ".into(), String::new());
        assert_eq!(input.name, "Ops");
        assert!(input.taxonomy.is_none());
        assert!(input.description.is_none());
        let filled = tag_create_input("Ops".into(), "team".into(), "On call".into());
        assert_eq!(filled.taxonomy.as_deref(), Some("team"));
        assert_eq!(filled.description.as_deref(), Some("On call"));
    }
}
