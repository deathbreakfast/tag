//! Reusable privacy policy rules for tag Valence schemas.

use async_trait::async_trait;
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

/// `service_name` prefix lepton gives a bot's `Actor::ServiceUser`.
const BOT_SERVICE_PREFIX: &str = "bot:";

/// Privacy policy allowing a read when the actor is an active bot that acts
/// for a user, so the bot sees the shared catalog the way its owner does.
///
/// The actor must be an `Actor::ServiceUser` named `bot:<id>` whose stored
/// `bot` row has `status` `active`, `owner_kind` `user` and a `user` owner.
/// Any other service user (ingest or infra workers) is denied. Use it on
/// read only: writes stay with the owner rules.
///
/// The `bot` row is read raw as the caller, because bots cannot read that
/// table through privacy and the check must not elevate to look. The table
/// and field names are lepton's; this crate does not depend on lepton's bot
/// types.
#[derive(Debug, Clone, Copy)]
pub struct TagReadActiveBot;

#[async_trait]
impl PolicyEvaluator for TagReadActiveBot {
    fn name(&self) -> &'static str {
        "tag::TAG_READ_ACTIVE_BOT"
    }

    fn description(&self) -> Option<&'static str> {
        Some("Allow read when the actor is an active bot owned by a user")
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
        Ok(active_bot_owner(&actor, v).await?.is_some())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Shared [`TagReadActiveBot`] instance for use in schema `privacy_policies` declarations.
pub const TAG_READ_ACTIVE_BOT: TagReadActiveBot = TagReadActiveBot;

/// Bare owner id of the active bot `actor` acts as, or `None` when `actor` is
/// not a bot, the bot is missing or not active, or it has no user owner.
async fn active_bot_owner(actor: &Actor, v: &Valence) -> valence::Result<Option<String>> {
    let Actor::ServiceUser { service_name } = actor else {
        return Ok(None);
    };
    let Some(bot_id) = service_name
        .strip_prefix(BOT_SERVICE_PREFIX)
        .filter(|id| !id.is_empty())
    else {
        return Ok(None);
    };
    let backend = v
        .backend_for_table("bot")
        .map_err(|e| Error::Privacy(format!("resolve bot backend: {e}")))?;
    let Some(row) = backend
        .get_record("bot", bot_id)
        .await
        .map_err(|e| Error::Privacy(format!("read bot: {e}")))?
    else {
        return Ok(None);
    };
    let active = row.get("status").and_then(|s| s.as_str()) == Some("active");
    let user_owned = row.get("owner_kind").and_then(|s| s.as_str()) == Some("user");
    if !(active && user_owned) {
        return Ok(None);
    }
    Ok(row
        .get("user")
        .and_then(|user| valence::extract_id_from_select_value(user).ok())
        .filter(|id| !id.trim().is_empty()))
}
