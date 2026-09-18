#![cfg(feature = "ssr")]
#![allow(missing_docs)]
#![allow(clippy::expect_used)]
#![allow(dead_code)]

use std::sync::Arc;

use chrono::Utc;
use lepton_identity::generated::{User, UserStatus, UserUserType};
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

pub const TEST_USER_A: &str = "tag-test-user-a";
pub const TEST_USER_B: &str = "tag-test-user-b";

pub async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    // Drop process-wide point-get cache so prior tests cannot satisfy `get` on a fresh DB.
    valence::clear_for_test();

    // Unified ownership fetch emits Surreal-shaped RETURN SQL that SQLite rejects.
    // Colocate stays at the Valence default (on) so ownership rows share the model store.
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: test harness only; OnceLock reads this before first ownership get.
        unsafe {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("SqliteBackend::connect_memory"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        backend,
        &["default"],
        RegisterBackendLogicalNamesOptions::default(),
    );

    Valence::builder()
        .database_router(Arc::new(router))
        .default_backend_key(valence::router_key("default", SQLITE_ENGINE_ID))
        .with_actor(Actor::System {
            operation: "tag_test".to_string(),
        })
        .build()
        .expect("build valence")
}

pub async fn seed_user(id: &str, email: &str, valence: &Valence) {
    let _ = email; // email lives on AccountEmail upstream; label kept for call-site readability
    let now = Utc::now();
    let user = User::new(
        Some(UserUserType::Person),
        Some("test-password-hash".to_string()),
        Some(UserStatus::Active),
        None,
        None,
        Some(now),
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    User::upsert_used(id, user, valence, valence::use_!(r"**Test:** Fixture **User** save for `tests` so the suite can arrange and assert persistence behavior. CI and developers running the suite only.")).await.expect("upsert user");
}

/// One shared `:memory:` DB with both fixture users seeded (System actor).
///
/// Cross-actor privacy tests must call [`as_user`] on this handle — never
/// `setup_valence()` twice (that yields isolated DBs and false assurance).
pub async fn setup_shared_db() -> Valence {
    let v = setup_valence().await;
    seed_user(TEST_USER_A, "owner-a@test.local", &v).await;
    seed_user(TEST_USER_B, "owner-b@test.local", &v).await;
    v
}

/// Same shared DB as `base`, switched to the given user actor.
pub fn as_user(base: &Valence, user_id: &str) -> Valence {
    base.with_actor(Actor::User {
        user_id: user_id.to_string(),
    })
}

/// Convenience for single-actor tests: shared fixture DB as `user_id`.
pub async fn valence_for(user_id: &str) -> Valence {
    let v = setup_shared_db().await;
    as_user(&v, user_id)
}

pub async fn execute_deletion_dag(valence: &Valence, table: &str, bare_id: &str) {
    use valence::deletion::dag::DeletionDag;

    let dag = DeletionDag::compute(table, bare_id, valence)
        .await
        .expect("compute dag");
    assert!(
        dag.restrict_violations.is_empty(),
        "restrict violations: {:?}",
        dag.restrict_violations
    );
    for node in &dag.nodes {
        let backend = valence
            .backend_for_table(&node.table)
            .expect("backend for deletion step");
        backend
            .delete_record(&node.table, &node.record_id)
            .await
            .expect("delete dag node");
    }
}
