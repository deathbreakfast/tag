//! [`valence::SideEffect`] that appends `tag_history` audit rows on every
//! [`crate::generated::Tag`] mutation.

use async_trait::async_trait;
use chrono::Utc;
use valence::{Model, Mutation, MutationKind, RecordId, SideEffect};

use crate::generated::{Tag, TagHistory};

/// Writes one [`TagHistory`] row per changed field (or a single `created`/`deleted`
/// row) whenever a [`Tag`] is created, updated, or deleted.
pub struct TagHistoryWriter;

fn tag_source_id(tag_id: &str) -> RecordId {
    RecordId::new("tag", tag_id)
}

fn actor_record(actor: &valence::Actor) -> Option<RecordId> {
    actor.user_id().map(|uid| {
        let bare = valence::ownership::normalize_record_id_for_ownership(uid);
        RecordId::new("user", bare)
    })
}

fn display_opt(value: Option<&Option<String>>) -> String {
    value
        .and_then(|inner| inner.as_ref())
        .map_or("", String::as_str)
        .to_string()
}

/// Resolve tag id from mutation snapshot; callers may pass an explicit id when known.
fn resolve_tag_id(mutation: &Mutation<'_, Tag>, explicit: Option<&str>) -> anyhow::Result<String> {
    if let Some(id) = explicit.filter(|s| !s.is_empty()) {
        return Ok(id.to_string());
    }
    let from_model = mutation
        .after()
        .or_else(|| mutation.before())
        .and_then(|t| t.id().and_then(|r| valence::extract_id_from_record(r).ok()))
        .filter(|s| !s.is_empty());
    from_model
        .ok_or_else(|| anyhow::anyhow!("tag history: missing tag record id on mutation snapshot"))
}

async fn append_row(
    source: RecordId,
    field_name: &str,
    old_value: &str,
    new_value: &str,
    subject_display_name: Option<String>,
    actor: Option<RecordId>,
    valence: &valence::Valence,
) -> anyhow::Result<()> {
    // Append under the session/mutation actor. `tag_history` create defers to
    // parent Tag Update — no mid-request System elevate.
    let row = TagHistory::new(
        subject_display_name,
        source.clone(),
        field_name.to_string(),
        old_value.to_string(),
        new_value.to_string(),
        Utc::now(),
        actor,
    )?;
    if let Err(e) = TagHistory::create(row, valence, valence::use_!(r"When **side_effects** needs to persist work, we **save Tag History** so the next step in that feature can continue with the latest values. People and services allowed for **side_effects** use this data for that workflow—not as a general export of unrelated personal fields.")).await {
        log::warn!("tag history append failed: source={source} field={field_name}: {e}");
        return Err(e.into());
    }
    Ok(())
}

#[async_trait]
impl SideEffect<Tag> for TagHistoryWriter {
    async fn on_mutation(&self, mutation: &Mutation<'_, Tag>) -> valence::Result<()> {
        self.on_mutation_with_tag_id(mutation, None)
            .await
            .map_err(|e| valence::Error::Internal(e.to_string()))
    }
}

impl TagHistoryWriter {
    /// Write history for a tag mutation. Pass `explicit_tag_id` when the upsert/update key is known
    /// (e.g. tag service create) so rows are not stored under an empty `source`.
    pub async fn on_mutation_with_tag_id(
        &self,
        mutation: &Mutation<'_, Tag>,
        explicit_tag_id: Option<&str>,
    ) -> anyhow::Result<()> {
        let actor = actor_record(mutation.valence().actor());
        let tag_id = resolve_tag_id(mutation, explicit_tag_id)?;
        let source = tag_source_id(&tag_id);
        let valence = mutation.valence();

        match *mutation.kind() {
            MutationKind::Create => {
                let after = mutation
                    .after()
                    .ok_or_else(|| anyhow::anyhow!("create mutation missing after snapshot"))?;
                append_row(
                    source,
                    "created",
                    "",
                    after.name(),
                    Some(after.name().clone()),
                    actor,
                    valence,
                )
                .await?;
            }
            MutationKind::Update => {
                let fields = mutation.fields();
                if fields.name.has_changed() {
                    append_row(
                        source.clone(),
                        "name",
                        fields.name.before().map_or("", String::as_str),
                        fields.name.after().map_or("", String::as_str),
                        fields.name.after().cloned(),
                        actor.clone(),
                        valence,
                    )
                    .await?;
                }
                if fields.taxonomy.has_changed() {
                    append_row(
                        source.clone(),
                        "taxonomy",
                        &display_opt(fields.taxonomy.before()),
                        &display_opt(fields.taxonomy.after()),
                        None,
                        actor.clone(),
                        valence,
                    )
                    .await?;
                }
                if fields.description.has_changed() {
                    append_row(
                        source.clone(),
                        "description",
                        &display_opt(fields.description.before()),
                        &display_opt(fields.description.after()),
                        None,
                        actor.clone(),
                        valence,
                    )
                    .await?;
                }
            }
            MutationKind::Delete => {
                let before = mutation.before();
                append_row(
                    source,
                    "deleted",
                    "",
                    before.map_or("", |t| t.name().as_str()),
                    before.map(|t| t.name().clone()),
                    actor,
                    valence,
                )
                .await?;
            }
        }
        Ok(())
    }
}
