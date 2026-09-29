#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

mod helpers;

use helpers::{valence_for, TEST_USER_A};
use tag::types::{TagCreateInput, TagUpdateInput};
use tag::{create, get_by_name, list, update, TagError};

fn named(name: &str) -> TagCreateInput {
    TagCreateInput {
        name: name.into(),
        taxonomy: None,
        description: None,
    }
}

fn rename(name: &str) -> TagUpdateInput {
    TagUpdateInput {
        name: Some(name.into()),
        taxonomy: None,
        description: None,
    }
}

async fn history_count(v: &valence::Valence) -> usize {
    let backend = v
        .backend_for_table("tag_history")
        .expect("tag_history backend");
    let all = valence::__internal::CompiledQuery::new("SELECT * FROM tag_history".into(), vec![]);
    backend
        .execute_compiled_query(&all)
        .await
        .map_or(0, |rows| rows.len())
}

#[tokio::test]
async fn create_duplicate_name_case_insensitive_sad() {
    let v = valence_for(TEST_USER_A).await;
    create(named("Ops"), &v).await.expect("first create");
    let history_before = history_count(&v).await;

    for variant in ["ops", " OPS ", "oPs"] {
        let err = create(named(variant), &v)
            .await
            .expect_err("duplicate name rejected");
        assert!(
            matches!(err, TagError::DuplicateName { ref name_key } if name_key == "ops"),
            "{variant:?} gave {err:?}"
        );
    }

    let rows = list(&v, None, None).await.expect("list");
    assert_eq!(rows.len(), 1, "no second tag row");
    assert_eq!(history_count(&v).await, history_before, "no history row");
}

#[tokio::test]
async fn update_to_existing_name_sad() {
    let v = valence_for(TEST_USER_A).await;
    create(named("Finance"), &v).await.expect("create finance");
    let travel = create(named("Travel"), &v).await.expect("create travel");

    let err = update(&travel.id, rename("finance "), &v)
        .await
        .expect_err("rename onto existing name rejected");
    assert!(matches!(err, TagError::DuplicateName { ref name_key } if name_key == "finance"));

    let still = get_by_name("travel", &v)
        .await
        .expect("lookup")
        .expect("row");
    assert_eq!(still.name, "Travel");
}

#[tokio::test]
async fn update_same_name_different_case_happy() {
    let v = valence_for(TEST_USER_A).await;
    let ops = create(named("ops"), &v).await.expect("create");
    let renamed = update(&ops.id, rename("OPS"), &v)
        .await
        .expect("case-only rename of own name");
    assert_eq!(renamed.name, "OPS");
}

#[tokio::test]
async fn get_by_name_normalizes_happy() {
    let v = valence_for(TEST_USER_A).await;
    let ops = create(named("Ops"), &v).await.expect("create");
    for query in ["Ops", "OPS", "  ops  "] {
        let found = get_by_name(query, &v).await.expect("lookup");
        assert_eq!(found.map(|t| t.id), Some(ops.id.clone()), "{query:?}");
    }
    assert!(get_by_name("missing", &v).await.expect("lookup").is_none());
    assert!(get_by_name("   ", &v).await.expect("lookup").is_none());
}

#[tokio::test]
async fn concurrent_create_same_name_one_wins_sad() {
    const RACERS: usize = 8;
    let v = valence_for(TEST_USER_A).await;
    let mut tasks = Vec::with_capacity(RACERS);
    for _ in 0..RACERS {
        let v = v.clone();
        tasks.push(tokio::spawn(async move { create(named("Race"), &v).await }));
    }

    let mut winners = 0;
    for task in tasks {
        match task.await.expect("join") {
            Ok(_) => winners += 1,
            Err(TagError::DuplicateName { name_key }) => assert_eq!(name_key, "race"),
            Err(other) => panic!("loser must get DuplicateName, got {other:?}"),
        }
    }
    assert_eq!(winners, 1);
    let rows = list(&v, Some("Race".into()), None).await.expect("list");
    assert_eq!(rows.len(), 1);
}
