//! In-memory `SQLite` Valence for server-side unit tests.
#![allow(clippy::expect_used)]

use std::sync::Arc;

use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

pub const TEST_USER: &str = "tag-app-test-user";

/// Fresh `:memory:` database routed for the tag schemas, as [`TEST_USER`].
pub async fn valence_as_user() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // Unified ownership fetch emits Surreal-shaped SQL that SQLite rejects.
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
        // SAFETY: test harness only; read once before the first ownership get.
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
        tag::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions::default(),
    );
    Valence::builder()
        .database_router(Arc::new(router))
        .default_backend_key(valence::router_key(
            tag::embedded_surreal::DEFAULT_LOGICAL_NAME,
            SQLITE_ENGINE_ID,
        ))
        .with_actor(Actor::User {
            user_id: TEST_USER.to_string(),
        })
        .build()
        .expect("build valence")
}
