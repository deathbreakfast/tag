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
