#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod helpers;

use chrono::Utc;
use helpers::{as_user, setup_shared_db, valence_for, TEST_USER_A, TEST_USER_B};
use tag::generated::TagHistory;
use tag::types::{TagCreateInput, TagUpdateInput};
use tag::{create, update};
use valence::{Model, PrivacyEvaluator, PrivacyOperation, QueryCore, RecordId, SchemaRegistry};

#[tokio::test]
async fn tag_history_create_defers_to_tag_update_owner_happy() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Owner Hist".into(),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect("create tag");

    let row = TagHistory::new(
        Some("extra".into()),
        RecordId::new("tag", &created.id),
        "note".to_string(),
        String::new(),
        "x".to_string(),
        Utc::now(),
        Some(RecordId::new("user", TEST_USER_A)),
    )
    .expect("build");
    TagHistory::create(row, &v, valence::use_!(r"**Test:** Fixture **Tag History** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."))
        .await
        .expect("owner may create history via defer→Tag Update");
}

#[tokio::test]
async fn tag_history_create_non_owner_forge_denied_sad() {
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let outsider_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Audit Subject".into(),
            taxonomy: None,
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create tag");

    let forged = TagHistory::new(
        Some("Forged".into()),
        RecordId::new("tag", &created.id),
        "forged_field".to_string(),
        String::new(),
        "spoof".to_string(),
        Utc::now(),
        Some(RecordId::new("user", TEST_USER_A)),
    )
    .expect("build forged row");
    let forge_attempt = TagHistory::create(forged, &outsider_v, valence::use_!(r"**Test:** Fixture **Tag History** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only.")).await;
    assert!(
        forge_attempt.is_err(),
        "outsider TagHistory::create must fail without Tag Update"
    );
}

#[tokio::test]
async fn tag_history_writer_appends_as_session_actor_happy() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Writer Subject".into(),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect("create");

    update(
        &created.id,
        TagUpdateInput {
            name: Some("Writer Subject Renamed".into()),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect("update must append history under session Valence");

    let backend = v
        .backend_for_table("tag_history")
        .expect("tag_history backend");
    let all = valence::__internal::CompiledQuery::new("SELECT * FROM tag_history".into(), vec![]);
    let rows = backend
        .execute_compiled_query(&all)
        .await
        .expect("history scan");
    let name_row = rows
        .iter()
        .find(|r| r.get("field_name").and_then(|v| v.as_str()) == Some("name"))
        .expect("name history row from session writer");
    let actor = name_row
        .get("actor")
        .expect("history row should record requesting user");
    let actor_id = match actor {
        serde_json::Value::String(s) => valence::extract_id_from_record_display(s).ok(),
        serde_json::Value::Object(o) => o.get("id").and_then(|v| v.as_str()).map(str::to_string),
        _ => None,
    };
    assert_eq!(actor_id.as_deref(), Some(TEST_USER_A));
}

#[tokio::test]
async fn tag_history_update_non_owner_denied_sad() {
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let outsider_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Upd Deny".into(),
            taxonomy: None,
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create");

    let system = base.with_actor(valence::Actor::System {
        operation: "tag_hist_probe".into(),
    });
    // Find a history row id from create.
    let backend = system.backend_for_table("tag_history").expect("backend");
    let all = valence::__internal::CompiledQuery::new("SELECT * FROM tag_history".into(), vec![]);
    let rows = backend.execute_compiled_query(&all).await.expect("scan");
    let hist_id = rows
        .iter()
        .find(|r| {
            r.get("source")
                .is_some_and(|s| s.to_string().contains(&created.id))
                || r.get("source_id").and_then(|v| v.as_str()) == Some(created.id.as_str())
        })
        .and_then(|r| r.get("id"))
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => {
                o.get("id").and_then(|x| x.as_str()).map(str::to_string)
            }
            _ => None,
        })
        .expect("history id");
    let raw = QueryCore::get_record_json(
        "tag_history",
        &hist_id,
        &system,
        valence::use_!(r"**Test:** Fixture row load for `privacy_policy_integration` so the suite can assert Valence privacy policy allow and deny outcomes. CI and developers running the suite only."),
    )
    .await
    .expect("get")
    .expect("row");
    let schema = SchemaRegistry::global()
        .get_schema("tag_history")
        .expect("schema");
    assert!(
        PrivacyEvaluator::check_entity_access(schema, PrivacyOperation::Update, &raw, &outsider_v)
            .await
            .is_err(),
        "outsider must not update tag_history"
    );
}

#[tokio::test]
async fn tag_history_delete_non_owner_denied_sad() {
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let outsider_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Del Deny".into(),
            taxonomy: None,
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create");

    let system = base.with_actor(valence::Actor::System {
        operation: "tag_hist_del_probe".into(),
    });
    let backend = system.backend_for_table("tag_history").expect("backend");
    let all = valence::__internal::CompiledQuery::new("SELECT * FROM tag_history".into(), vec![]);
    let rows = backend.execute_compiled_query(&all).await.expect("scan");
    let hist_id = rows
        .iter()
        .find(|r| {
            r.get("source")
                .is_some_and(|s| s.to_string().contains(&created.id))
        })
        .and_then(|r| r.get("id"))
        .and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Object(o) => {
                o.get("id").and_then(|x| x.as_str()).map(str::to_string)
            }
            _ => None,
        })
        .expect("history id");

    let err = TagHistory::delete(&hist_id, &outsider_v, valence::use_!(r"**Test:** Fixture **Tag History** remove for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only.")).await;
    assert!(
        err.is_err(),
        "outsider must not delete tag_history without Tag Delete"
    );
}

#[tokio::test]
async fn tag_update_delete_remain_owner_or_system_only_sad() {
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let outsider_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Owner Only".into(),
            taxonomy: None,
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create");

    let upd = update(
        &created.id,
        TagUpdateInput {
            name: Some("Hijacked".into()),
            taxonomy: None,
            description: None,
        },
        &outsider_v,
    )
    .await;
    assert!(upd.is_err(), "outsider must not update tag");

    let del = tag::delete(&created.id, &outsider_v).await;
    assert!(del.is_err(), "outsider must not delete tag");
}

#[tokio::test]
async fn tag_history_schema_has_source_edge_for_defer() {
    let schema = SchemaRegistry::global()
        .get_schema("tag_history")
        .expect("tag_history registered");
    let conn_names: Vec<_> = schema
        .schema
        .connections
        .iter()
        .map(|c| c.name.as_str())
        .collect();
    let field_names: Vec<_> = schema
        .schema
        .fields
        .iter()
        .map(|f| format!("{}:{}", f.name, f.field_type))
        .collect();
    assert!(
        conn_names.contains(&"source")
            || schema.schema.fields.iter().any(|f| {
                f.name == "source" && (f.fk.is_some() || f.field_type.contains("record"))
            }),
        "expected source edge/field; connections={conn_names:?} fields={field_names:?}"
    );
}

// Service users act as `Actor::ServiceUser { service_name: "svc:<id>" }`. The
// `service_user_account` table belongs to lepton and is not registered in this test graph, so
// the fixture writes its rows raw, the way Lepton's resolver reads them.

const ACTIVE_SERVICE_USER: &str = "3f2b8c1e-5d4a-4e7b-9c10-0a1b2c3d4e5f";
const SUSPENDED_SERVICE_USER: &str = "7a9d2e4b-1c3f-4a6e-8b5d-2e4f6a8c0b1d";
const OWNERLESS_SERVICE_USER: &str = "c4e6a8b0-2d4f-4a1c-9e3b-5d7f9a1b3c5e";
const MISSING_SERVICE_USER: &str = "00000000-0000-4000-8000-000000000000";

async fn seed_service_user(system: &valence::Valence, id: &str, has_owner: bool, status: &str) {
    let backend = system
        .backend_for_table("service_user_account")
        .expect("service user backend");
    let mut row = serde_json::json!({
        "id": id,
        "owner_kind": "user",
        "handle": &id[..8],
        "handle_key": &id[..8],
        "display_name": "Test service user",
        "status": status,
        "created_at": Utc::now().timestamp(),
        "updated_at": Utc::now().timestamp(),
    });
    // The resolver finds the owner through `user`, so a row without one resolves to nothing.
    if has_owner {
        row["user"] =
            serde_json::to_value(RecordId::new("user", TEST_USER_A)).expect("owner record");
    }
    backend
        .upsert_record("service_user_account", id, row)
        .await
        .expect("seed service user");
}

fn as_service(base: &valence::Valence, service_name: &str) -> valence::Valence {
    base.with_actor(valence::Actor::ServiceUser {
        service_name: service_name.to_string(),
    })
}

/// Shared DB with the service user fixtures and one tag created by `TEST_USER_A`.
async fn service_user_fixture(name: &str) -> (valence::Valence, String) {
    let base = setup_shared_db().await;
    seed_service_user(&base, ACTIVE_SERVICE_USER, true, "active").await;
    seed_service_user(&base, SUSPENDED_SERVICE_USER, true, "suspended").await;
    seed_service_user(&base, OWNERLESS_SERVICE_USER, false, "active").await;
    let created = create(
        TagCreateInput {
            name: name.into(),
            taxonomy: Some("spend".into()),
            description: None,
        },
        &as_user(&base, TEST_USER_A),
    )
    .await
    .expect("owner creates tag");
    (base, created.id)
}

/// The actor cannot see the tag: `get` finds nothing and `list` leaves it out.
async fn assert_cannot_read(v: &valence::Valence, tag_id: &str, who: &str) {
    let got = tag::get(tag_id, v)
        .await
        .expect("denied get is not an error");
    assert!(got.is_none(), "{who} must not read the tag, got {got:?}");
    let rows = tag::list(v, None, None)
        .await
        .expect("denied list is not an error");
    assert!(
        rows.iter().all(|r| r.id != tag_id),
        "{who} list must not include the tag"
    );
}

/// A write refused by Valence privacy, as the service reports it.
fn is_privacy_denial<T>(result: &Result<T, tag::TagError>) -> bool {
    match result {
        Err(tag::TagError::AccessDenied { .. }) => true,
        Err(tag::TagError::Service { source, .. }) => source.to_string().contains("Access denied"),
        _ => false,
    }
}

#[tokio::test]
async fn active_service_user_reads_and_lists_shared_tag_happy_path() {
    let (base, tag_id) = service_user_fixture("Service user Visible").await;
    let service_user = as_service(&base, &format!("svc:{ACTIVE_SERVICE_USER}"));

    let seen = tag::get(&tag_id, &service_user)
        .await
        .expect("service user get")
        .expect("active service user must read the shared catalog tag");
    assert_eq!(seen.name, "Service user Visible");

    let listed = tag::list(&service_user, None, None)
        .await
        .expect("service user list");
    assert!(
        listed
            .iter()
            .any(|r| r.id == tag_id && r.name == "Service user Visible"),
        "active service user list must include the owner-created tag"
    );
}

#[tokio::test]
async fn suspended_ownerless_or_missing_service_user_cannot_read_tag_sad() {
    let (base, tag_id) = service_user_fixture("Service user Hidden").await;
    for (service_user_id, who) in [
        (SUSPENDED_SERVICE_USER, "suspended service user"),
        (OWNERLESS_SERVICE_USER, "service user without an owner"),
        (MISSING_SERVICE_USER, "missing service user"),
    ] {
        let service_user = as_service(&base, &format!("svc:{service_user_id}"));
        assert_cannot_read(&service_user, &tag_id, who).await;
    }
}

#[tokio::test]
async fn builtin_service_user_cannot_read_tag_sad() {
    let (base, tag_id) = service_user_fixture("Infra Hidden").await;
    for service_name in ["spectra-ingest", "cron", "svc:", "robot:3f2b8c1e"] {
        let infra = as_service(&base, service_name);
        assert_cannot_read(&infra, &tag_id, service_name).await;
    }
}

/// A service name that is not the canonical `svc:<uuid>` form gets nothing, even
/// when the uuid inside it names an active service user.
#[tokio::test]
async fn non_canonical_service_user_name_cannot_read_tag_sad() {
    let (base, tag_id) = service_user_fixture("Non Canonical Hidden").await;
    for name in [
        format!("svc:{}", ACTIVE_SERVICE_USER.to_uppercase()),
        format!("svc: {ACTIVE_SERVICE_USER}"),
        format!("svc:{ACTIVE_SERVICE_USER}/x"),
    ] {
        assert_cannot_read(&as_service(&base, &name), &tag_id, &name).await;
    }
}

/// The retired `bot:<uuid>` name gets nothing, even for a uuid whose service
/// user row is active.
#[tokio::test]
async fn legacy_bot_actor_cannot_read_tag_sad() {
    let (base, tag_id) = service_user_fixture("Legacy Hidden").await;
    let legacy = format!("bot:{ACTIVE_SERVICE_USER}");
    assert_cannot_read(&as_service(&base, &legacy), &tag_id, &legacy).await;
}

#[tokio::test]
async fn active_service_user_cannot_create_or_update_tag_sad() {
    let (base, tag_id) = service_user_fixture("Service user Read Only").await;
    let service_user = as_service(&base, &format!("svc:{ACTIVE_SERVICE_USER}"));

    let created = create(
        TagCreateInput {
            name: "Filed By Service user".into(),
            taxonomy: None,
            description: None,
        },
        &service_user,
    )
    .await;
    assert!(
        is_privacy_denial(&created),
        "service user must not create a tag: {created:?}"
    );

    let updated = update(
        &tag_id,
        TagUpdateInput {
            name: Some("Renamed By Service user".into()),
            taxonomy: None,
            description: None,
        },
        &service_user,
    )
    .await;
    assert!(
        is_privacy_denial(&updated),
        "service user must not update its owner's tag: {updated:?}"
    );

    let owner_view = tag::get(&tag_id, &as_user(&base, TEST_USER_A))
        .await
        .expect("owner get")
        .expect("tag still exists");
    assert_eq!(owner_view.name, "Service user Read Only");
    let filed = tag::get_by_name("Filed By Service user", &as_user(&base, TEST_USER_A))
        .await
        .expect("owner lookup");
    assert!(
        filed.is_none(),
        "denied service user create must not leave a row"
    );
}
