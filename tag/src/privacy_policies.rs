//! Reusable privacy policy rules for tag Valence schemas.

use async_trait::async_trait;
use lepton_identity::service_users::active_service_user_owner;
use std::any::Any;
use valence::ownership::OwnershipService;
use valence::{Actor, ActorContext, Error, PolicyEvaluator, PrivacyOperation, Valence};

/// Privacy policy allowing access when the `valence_data_ownership` owner
/// matches the requesting actor's user id.
#[derive(Debug, Clone, Copy)]
pub struct TagDataOwner;

#[async_trait]
impl PolicyEvaluator for TagDataOwner {
    fn name(&self) -> &'static str {
        "tag::TAG_DATA_OWNER"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Allow when valence_data_ownership owner matches the actor user")
    }

    async fn evaluate(
        &self,
        _op: PrivacyOperation,
        record: &serde_json::Value,
        actor: &dyn ActorContext,
        v: &Valence,
    ) -> valence::Result<bool> {
        let actor: Actor = serde_json::from_value(actor.actor_json().clone())
            .map_err(|e| Error::Internal(format!("invalid actor context: {e}")))?;
        let Some(user_id) = actor.user_id() else {
            return Ok(false);
        };
        // SQLite/mem rows may store `id` as a string or as `{ table, id }`.
        let Some(record_id) = record
            .get("id")
            .and_then(|v| valence::extract_id_from_select_value(v).ok())
        else {
            return Ok(false);
        };
        let actor_owner = valence::ownership::normalize_record_id_for_ownership(user_id);
        let bare = valence::ownership::normalize_record_id_for_ownership(&record_id);
        let ownership = OwnershipService::get_ownership_json("tag", &bare, v).await?;
        let Some(row) = ownership else {
            return Ok(false);
        };
        Ok(row
            .get("owner_id")
            .and_then(|v| v.as_str())
            .map(valence::ownership::normalize_record_id_for_ownership)
            == Some(actor_owner))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Shared [`TagDataOwner`] instance for use in schema `privacy_policies` declarations.
pub const TAG_DATA_OWNER: TagDataOwner = TagDataOwner;

/// Privacy policy allowing a read when the actor is an active service user that acts
/// for a user, so the service user sees the shared catalog the way its owner does.
///
/// The actor must be an `Actor::ServiceUser` that Lepton's
/// [`active_service_user_owner`] resolves to an owning user: a canonical `svc:<id>`
/// name whose stored row is `active`, not pending deletion, and has a `user` owner.
/// Any other service user (ingest or infra workers) is denied. Use it on read only:
/// writes stay with the owner rules.
///
/// Lepton's `SERVICE_USER_ACTIVE` rule matches rows through a `user` field, which tag
/// rows do not have (ownership lives in `valence_data_ownership`), so this rule
/// calls the resolver directly. The resolver reads the row from the backend, so the
/// check does not elevate the caller.
#[derive(Debug, Clone, Copy)]
pub struct TagReadActiveServiceUser;

#[async_trait]
impl PolicyEvaluator for TagReadActiveServiceUser {
    fn name(&self) -> &'static str {
        "tag::TAG_READ_ACTIVE_SERVICE_USER"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Allow read when the actor is an active service user owned by a user")
    }

    async fn evaluate(
        &self,
        op: PrivacyOperation,
        _record: &serde_json::Value,
        actor: &dyn ActorContext,
        v: &Valence,
    ) -> valence::Result<bool> {
        if !matches!(op, PrivacyOperation::Read) {
            return Ok(false);
        }
        let actor: Actor = serde_json::from_value(actor.actor_json().clone())
            .map_err(|e| Error::Internal(format!("invalid actor context: {e}")))?;
        let Actor::ServiceUser { service_name } = &actor else {
            return Ok(false);
        };
        Ok(active_service_user_owner(service_name, v).await.is_some())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Shared [`TagReadActiveServiceUser`] instance for use in schema `privacy_policies` declarations.
pub const TAG_READ_ACTIVE_SERVICE_USER: TagReadActiveServiceUser = TagReadActiveServiceUser;
