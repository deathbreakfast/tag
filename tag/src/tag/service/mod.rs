//! Tag catalog CRUD (no product connection helpers).
//!
//! # Errors
//!
//! Fallible entry points return [`TagError`]: [`TagError::NotFound`] on update
//! of a missing id, [`TagError::AccessDenied`] when ownership blocks delete,
//! and [`TagError::Service`] for Valence / history failures. `get` uses
//! `Ok(None)` for absence. `tag-app` maps these into `ServerFnError`.

mod helpers;

use chrono::Utc;
use uuid::Uuid;
use valence::{Model, Mutation, MutationKind, StringPredicate, Valence};

use crate::generated::Tag;
use crate::side_effects::TagHistoryWriter;
use crate::types::{TagCreateInput, TagDetailDto, TagRowDto, TagUpdateInput};

use super::TagError;
use helpers::{
    ensure_caller_may_delete_tag, owner_display, record_id_str, to_detail_dto_with_id, to_row_dto,
};

/// Create a new tag and write its `created` history row.
pub async fn create(input: TagCreateInput, v: &Valence) -> Result<TagDetailDto, TagError> {
    let now = Utc::now();
    let id = Uuid::new_v4().to_string();
    let tag = Tag::new(input.name, input.taxonomy, input.description, now, now)
        .map_err(|e| TagError::service("create", e))?;
    let created = Tag::upsert_used(&id, tag, v, valence::use_!(r"When **service** needs to persist work, we **save Tag** so the next step in that feature can continue with the latest values. People and services allowed for **service** use this data for that workflow—not as a general export of unrelated personal fields."))
        .await
        .map_err(|e| TagError::service("create", e))?;
    let field_changes = crate::generated::TagFieldChanges::compute(None, Some(&created));
    let mutation = Mutation::new(
        MutationKind::Create,
        None,
        Some(created.clone()),
        field_changes,
        v,
    );
    TagHistoryWriter
        .on_mutation_with_tag_id(&mutation, Some(&id))
        .await
        .map_err(|e| TagError::service("create", e))?;
    let owner = owner_display(&id, v).await;
    Ok(to_detail_dto_with_id(&created, &id, owner))
}

/// Apply a partial update to an existing tag and write per-field history rows
/// for whichever fields actually changed.
pub async fn update(
    id: &str,
    input: TagUpdateInput,
    v: &Valence,
) -> Result<TagDetailDto, TagError> {
    let before = Tag::get_used(id, v, valence::use_!(r"In **service**, we **load Tag** so the application can decide what to do next in this workflow. The result is used by **service** logic—not necessarily displayed on a page unless that feature’s UI shows it."))
        .await
        .map_err(|e| TagError::service("update", e))?
        .ok_or_else(|| TagError::not_found(id))?;
    let mut builder = before.get_mutable_used(v, valence::use_!(r"In **service**, we **update Tag Mutable** in place so saved changes apply on the next read. The same actors who can run **service** use the updated values; this step is not a silent copy to an external marketing system."));
    if let Some(name) = input.name {
        builder = builder
            .set_name(name)
            .map_err(|e| TagError::service("update", e))?;
    }
    if let Some(taxonomy) = input.taxonomy {
        builder = builder
            .set_taxonomy(taxonomy)
            .map_err(|e| TagError::service("update", e))?;
    }
    if let Some(description) = input.description {
        builder = builder
            .set_description(description)
            .map_err(|e| TagError::service("update", e))?;
    }
    builder = builder
        .set_updated_at(Utc::now())
        .map_err(|e| TagError::service("update", e))?;
    let updated = builder
        .commit()
        .await
        .map_err(|e| TagError::service("update", e))?;
    let field_changes = crate::generated::TagFieldChanges::compute(Some(&before), Some(&updated));
    let mutation = Mutation::new(
        MutationKind::Update,
        Some(before),
        Some(updated.clone()),
        field_changes,
        v,
    );
    TagHistoryWriter
        .on_mutation_with_tag_id(&mutation, Some(id))
        .await
        .map_err(|e| TagError::service("update", e))?;
    let owner = owner_display(id, v).await;
    Ok(to_detail_dto_with_id(&updated, id, owner))
}

/// Delete a tag by id and write its `deleted` history row.
///
/// Authorizes Delete for the caller (owner / System), appends history under the
/// session actor, then deletes the tag with the same actor so
/// `HistorySource` cascade can clear `tag_history` via delete `defer_to_edge`
/// (parent Delete) — no System elevate.
pub async fn delete(id: &str, v: &Valence) -> Result<(), TagError> {
    let Some(before) = Tag::get_used(id, v, valence::use_!(r"In **service**, we **load Tag** so the application can decide what to do next in this workflow. The result is used by **service** logic—not necessarily displayed on a page unless that feature’s UI shows it."))
        .await
        .map_err(|e| TagError::service("delete", e))?
    else {
        return Ok(());
    };
    ensure_caller_may_delete_tag(id, v).await?;
    let field_changes = crate::generated::TagFieldChanges::compute(Some(&before), None);
    let mutation = valence::Mutation::new(
        valence::MutationKind::Delete,
        Some(before),
        None,
        field_changes,
        v,
    );
    TagHistoryWriter
        .on_mutation_with_tag_id(&mutation, Some(id))
        .await
        .map_err(|e| TagError::service("delete", e))?;
    Tag::delete_used(id, v, valence::use_!(r"When **service** finishes cleanup, we **remove Tag** so leftover rows do not remain after the operation. Only the cleanup path for **service** uses this step; it is not shown as a standalone end-user page by itself."))
        .await
        .map_err(|e| TagError::service("delete", e))?;
    Ok(())
}

/// Load a single tag by id as a [`TagDetailDto`], resolving its owner display label.
pub async fn get(id: &str, v: &Valence) -> Result<Option<TagDetailDto>, TagError> {
    let Some(tag) = Tag::get_used(id, v, valence::use_!(r"In **service**, we **load Tag** so the application can decide what to do next in this workflow. The result is used by **service** logic—not necessarily displayed on a page unless that feature’s UI shows it."))
        .await
        .map_err(|e| TagError::service("get", e))?
    else {
        return Ok(None);
    };
    let owner = owner_display(id, v).await;
    Ok(Some(to_detail_dto_with_id(&tag, id, owner)))
}

/// List tags, optionally filtered by a name-contains `search` term and/or exact
/// `taxonomy`, ordered by most-recently updated first.
pub async fn list(
    v: &Valence,
    search: Option<String>,
    taxonomy: Option<String>,
) -> Result<Vec<TagRowDto>, TagError> {
    let mut q = Tag::query_used(
        v,
        valence::use_!(
            r"In **service**, we **list Tag** so the product can show or process the matching set for this workflow. Callers allowed for **service** use the list; it is not a public dump of every field to anonymous visitors."
        ),
    );
    if let Some(term) = search.filter(|s| !s.trim().is_empty()) {
        q = q.where_name(StringPredicate::Contains(term));
    }
    if let Some(tax) = taxonomy.filter(|s| !s.trim().is_empty()) {
        q = q.where_taxonomy(StringPredicate::Equals(tax));
    }
    let rows = q
        .order_by_updated_at(valence::query::SortDirection::Desc)
        .await
        .map_err(|e| TagError::service("list", e))?;
    let mut out = Vec::with_capacity(rows.len());
    for tag in rows {
        let id = record_id_str(&tag);
        let owner = owner_display(&id, v).await;
        out.push(to_row_dto(&tag, owner));
    }
    Ok(out)
}
