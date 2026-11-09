//! Named happy/sad contracts for product-local tag catalog CRUD.
//!
//! Covers the same domain surface as `tag-app` server fns
//! (`create_tag` / `list_tags` / `get_tag` / `update_tag` / `delete_tag`),
//! which are thin wrappers over `tag::{create,list,get,update,delete}`.

#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod helpers;

use helpers::{as_user, setup_shared_db, valence_for, TEST_USER_A, TEST_USER_B};
use tag::types::{TagCreateInput, TagUpdateInput};
use tag::{create, delete, get, list, update, TagError};

#[tokio::test]
async fn create_tag_returns_detail_happy_path() {
    let v = valence_for(TEST_USER_A).await;
    let detail = create(
        TagCreateInput {
            name: "Contract Create".into(),
            taxonomy: Some("spend".into()),
            description: Some("created by contract".into()),
        },
        &v,
    )
    .await
    .expect("create");

    assert_ne!(detail.id, "");
    assert_eq!(detail.name, "Contract Create");
    assert_eq!(detail.taxonomy.as_deref(), Some("spend"));
    assert_eq!(detail.description.as_deref(), Some("created by contract"));
    assert_eq!(detail.owner_display, "you");
}

#[tokio::test]
async fn get_tag_round_trips_happy_path() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Round Trip".into(),
            taxonomy: None,
            description: Some("persist".into()),
        },
        &v,
    )
    .await
    .expect("create");

    let got = get(&created.id, &v)
        .await
        .expect("get")
        .expect("row present");
    assert_eq!(got.id, created.id);
    assert_eq!(got.name, "Round Trip");
    assert_eq!(got.description.as_deref(), Some("persist"));
}

#[tokio::test]
async fn list_tags_includes_created_happy_path() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Listed Tag".into(),
            taxonomy: Some("custom".into()),
            description: None,
        },
        &v,
    )
    .await
    .expect("create");

    let rows = list(&v, None, None).await.expect("list");
    let row = rows
        .iter()
        .find(|r| r.id == created.id)
        .expect("created tag in list");
    assert_eq!(row.name, "Listed Tag");
    assert_eq!(row.taxonomy.as_deref(), Some("custom"));
}

#[tokio::test]
async fn update_tag_changes_fields_happy_path() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Before".into(),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect("create");

    let updated = update(
        &created.id,
        TagUpdateInput {
            name: Some("After".into()),
            taxonomy: Some("spend".into()),
            description: Some("updated".into()),
        },
        &v,
    )
    .await
    .expect("update");

    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "After");
    assert_eq!(updated.taxonomy.as_deref(), Some("spend"));
    assert_eq!(updated.description.as_deref(), Some("updated"));
}

#[tokio::test]
async fn delete_tag_removes_from_catalog_happy_path() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Disposable".into(),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect("create");

    delete(&created.id, &v).await.expect("delete");

    let pending = get(&created.id, &v).await;
    assert!(
        pending.is_err(),
        "deleted tag must be blocked by pending-deletion gate"
    );

    // Noop deletion dispatcher leaves the row until DAG finalize (SQLite pending-list
    // post-filter uses a Surreal-shaped IN query that does not run here).
    helpers::execute_deletion_dag(&v, "tag", &created.id).await;

    let listed = list(&v, None, None).await.expect("list");
    assert!(
        !listed.iter().any(|r| r.id == created.id),
        "deleted tag must not appear in list after DAG finalize"
    );
}

#[tokio::test]
async fn tag_crud_workflow_create_list_get_update_delete_happy_path() {
    let v = valence_for(TEST_USER_A).await;

    let created = create(
        TagCreateInput {
            name: "Workflow Alpha".into(),
            taxonomy: Some("spend".into()),
            description: Some("step-1".into()),
        },
        &v,
    )
    .await
    .expect("create");
    let id = created.id.clone();

    let listed = list(&v, Some("Workflow".into()), Some("spend".into()))
        .await
        .expect("list");
    assert!(listed.iter().any(|r| r.id == id));

    let got = get(&id, &v).await.expect("get").expect("present");
    assert_eq!(got.name, "Workflow Alpha");

    let updated = update(
        &id,
        TagUpdateInput {
            name: Some("Workflow Beta".into()),
            taxonomy: Some("custom".into()),
            description: Some("step-4".into()),
        },
        &v,
    )
    .await
    .expect("update");
    assert_eq!(updated.name, "Workflow Beta");
    assert_eq!(updated.taxonomy.as_deref(), Some("custom"));

    let after_update = get(&id, &v).await.expect("get").expect("present");
    assert_eq!(after_update.name, "Workflow Beta");
    assert_eq!(after_update.description.as_deref(), Some("step-4"));

    delete(&id, &v).await.expect("delete");

    let pending = get(&id, &v).await;
    assert!(pending.is_err(), "deleted tag must not be readable");
    let msg = pending.unwrap_err().to_string();
    assert!(
        msg.contains("Pending deletion") || msg.contains("Access denied"),
        "expected pending-deletion or privacy deny after delete, got: {msg}"
    );

    helpers::execute_deletion_dag(&v, "tag", &id).await;
    let after_delete = list(&v, None, None).await.expect("list");
    assert!(!after_delete.iter().any(|r| r.id == id));
}

#[tokio::test]
async fn get_tag_unknown_id_is_none_sad() {
    let v = valence_for(TEST_USER_A).await;
    let got = get("missing-tag-id-does-not-exist", &v)
        .await
        .expect("get ok");
    assert!(got.is_none());
}

#[tokio::test]
async fn update_tag_unknown_id_not_found_sad() {
    let v = valence_for(TEST_USER_A).await;
    let err = update(
        "missing-tag-id-does-not-exist",
        TagUpdateInput {
            name: Some("Nope".into()),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect_err("update missing must fail");
    match err {
        TagError::NotFound { id } => {
            assert_eq!(id, "missing-tag-id-does-not-exist");
        }
        other => panic!("expected TagError::NotFound, got: {other}"),
    }
}

#[tokio::test]
async fn list_tags_search_miss_empty_sad() {
    let v = valence_for(TEST_USER_A).await;
    create(
        TagCreateInput {
            name: "Visible Only".into(),
            taxonomy: Some("spend".into()),
            description: None,
        },
        &v,
    )
    .await
    .expect("create");

    let filtered = list(&v, Some("zzz-no-match-term".into()), None)
        .await
        .expect("list");
    assert!(
        filtered.is_empty(),
        "search miss should yield empty list, got {}",
        filtered.len()
    );
}

#[tokio::test]
async fn authenticated_peer_can_read_and_list_other_users_tag_happy_path() {
    // Shared DB + distinct actors (TG-03): catalog read is TAG_SIGNED_IN_USER (TG-01).
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let peer_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Shared Catalog".into(),
            taxonomy: Some("spend".into()),
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create");

    let seen = get(&created.id, &peer_v)
        .await
        .expect("get")
        .expect("authenticated peer must read shared catalog tag");
    assert_eq!(seen.name, "Shared Catalog");
    assert_ne!(seen.owner_display, "you");

    let listed = list(&peer_v, None, None).await.expect("list");
    assert!(
        listed
            .iter()
            .any(|r| r.id == created.id && r.name == "Shared Catalog"),
        "peer list must include owner-created tag"
    );
}

#[tokio::test]
async fn non_owner_cannot_update_other_users_tag_sad() {
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let other_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Owner Locked".into(),
            taxonomy: None,
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create");

    let err = update(
        &created.id,
        TagUpdateInput {
            name: Some("Hacked".into()),
            taxonomy: None,
            description: None,
        },
        &other_v,
    )
    .await;
    assert!(err.is_err(), "non-owner update must fail");
    let still = get(&created.id, &owner_v)
        .await
        .expect("get")
        .expect("still owned");
    assert_eq!(still.name, "Owner Locked");
}

#[tokio::test]
async fn non_owner_cannot_delete_other_users_tag_sad() {
    let base = setup_shared_db().await;
    let owner_v = as_user(&base, TEST_USER_A);
    let other_v = as_user(&base, TEST_USER_B);

    let created = create(
        TagCreateInput {
            name: "Owner Protected".into(),
            taxonomy: None,
            description: None,
        },
        &owner_v,
    )
    .await
    .expect("create");

    let err = delete(&created.id, &other_v)
        .await
        .expect_err("non-owner delete must fail when row is readable but not owned");
    match err {
        TagError::AccessDenied { policy } => {
            assert_eq!(policy, "tag::TAG_DATA_OWNER");
        }
        other => panic!("expected TagError::AccessDenied, got: {other}"),
    }
    assert!(get(&created.id, &owner_v).await.expect("get").is_some());
}

#[tokio::test]
async fn delete_tag_pending_deletion_blocks_get_sad() {
    let v = valence_for(TEST_USER_A).await;
    let created = create(
        TagCreateInput {
            name: "Pending Gate".into(),
            taxonomy: None,
            description: None,
        },
        &v,
    )
    .await
    .expect("create");

    delete(&created.id, &v).await.expect("delete");

    let pending = get(&created.id, &v).await;
    let msg = pending.expect_err("deleted tag must error").to_string();
    assert!(
        msg.contains("Pending deletion") || msg.contains("Access denied"),
        "expected pending-deletion or privacy deny after delete, got: {msg}"
    );
}
