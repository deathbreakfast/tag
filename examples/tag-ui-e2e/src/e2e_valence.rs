//! Process-wide Valence + Higgs for Playwright (tag catalog + history).

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::{Arc, Mutex, OnceLock};

use chrono::Utc;
use higgs::actor_policy::external_actor_json_policy;
use higgs::{HiggsConfig, HiggsValenceFactory};
use lepton_identity::generated::{User, UserStatus, UserUserType};
use tag::types::TagCreateInput;
use valence::{
    register_backend_logical_names, router_key, Actor, DatabaseBackend, DatabaseRouter, Model,
    RegisterBackendLogicalNamesOptions, RouterValenceFactory, RouterValenceFactoryConfig,
    SqliteBackend, Valence, ValenceFactory, SQLITE_ENGINE_ID,
};

struct E2eState {
    router: Arc<DatabaseRouter>,
    higgs: Arc<HiggsConfig>,
    default_backend_key: String,
    fixtures: Mutex<FixtureIds>,
}

/// Stable fixture ids exposed to seed JSON / Playwright.
#[derive(Clone, Debug, Default)]
pub struct FixtureIds {
    pub tag_id: String,
    pub tag_name: String,
    pub peer_tag_id: String,
    pub peer_tag_name: String,
}

static E2E_STATE: OnceLock<Arc<E2eState>> = OnceLock::new();

struct HiggsFactory(RouterValenceFactory);

impl HiggsValenceFactory for HiggsFactory {
    fn build(&self, actor_json: &serde_json::Value) -> anyhow::Result<Valence> {
        self.0.build(actor_json).map_err(|e| anyhow::anyhow!("{e}"))
    }
}

fn prepare_env() {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    // SAFETY: host boot only.
    unsafe {
        if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
            std::env::set_var("VALENCE_OWNERSHIP_UNIFIED_FETCH", "0");
        }
    }
}

async fn seed_user(id: &str, email_verified: bool, valence: &Valence) {
    let now = Utc::now();
    let confirmed_at = email_verified.then_some(now);
    let user = User::new(
        Some(UserUserType::Person),
        Some("e2e-password-hash".to_string()),
        Some(UserStatus::Active),
        None,
        None,
        confirmed_at,
        None,
        None,
        now,
        now,
    )
    .expect("build user");
    User::upsert_used(id, user, valence, valence::use_!(r#"**Test:** Fixture **User** save for `e2e_valence` so the suite can arrange and assert persistence behavior. CI and developers running the suite only."#)).await.expect("upsert user");
}

/// Build shared Valence/Higgs once and seed baseline catalog fixtures.
pub async fn init_e2e_valence() {
    if E2E_STATE.get().is_some() {
        return;
    }

    prepare_env();

    // Ensure TagHistory schema inventory is linked and source defer edge is present.
    {
        use valence::SchemaRegistry;
        let schema = SchemaRegistry::global()
            .get_schema("tag_history")
            .expect("tag_history schema must be registered before e2e seed");
        let ok = schema.schema.connections.iter().any(|c| c.name == "source")
            || schema.schema.fields.iter().any(|f| {
                f.name == "source"
                    && (f.fk.is_some()
                        || f.field_type.contains("record")
                        || f.field_type.starts_with("Record"))
            });
        assert!(
            ok,
            "tag_history missing source edge/field for defer_to_edge (connections={}, fields={:?})",
            schema.schema.connections.len(),
            schema
                .schema
                .fields
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
        );
    }

    let backend: Arc<dyn DatabaseBackend> = Arc::new(
        SqliteBackend::connect_memory()
            .await
            .expect("SqliteBackend::connect_memory"),
    );
    let mut router = DatabaseRouter::new();
    register_backend_logical_names(
        &mut router,
        Arc::clone(&backend),
        tag::embedded_surreal::EMBEDDED_SURREAL_LOGICAL_NAMES,
        RegisterBackendLogicalNamesOptions::default(),
    );
    let router = Arc::new(router);
    let default_key = router_key(
        tag::embedded_surreal::DEFAULT_LOGICAL_NAME,
        SQLITE_ENGINE_ID,
    );

    let system = Valence::builder()
        .database_router(Arc::clone(&router))
        .default_backend_key(default_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_tag_host".into(),
        })
        .build()
        .expect("e2e Valence");

    seed_user("owner", true, &system).await;
    seed_user("peer", true, &system).await;
    seed_user("unverified", false, &system).await;

    let fixtures = bootstrap_fixtures(&system)
        .await
        .expect("bootstrap fixtures");

    let factory: Arc<dyn HiggsValenceFactory> = Arc::new(HiggsFactory(RouterValenceFactory::new(
        Arc::clone(&router),
        RouterValenceFactoryConfig::new(default_key.clone())
            .actor_json_policy(external_actor_json_policy()),
    )));
    let higgs = Arc::new(
        HiggsConfig::builder()
            .valence_factory_arc(factory)
            .build()
            .expect("e2e HiggsConfig"),
    );

    let state = Arc::new(E2eState {
        router,
        higgs,
        default_backend_key: default_key,
        fixtures: Mutex::new(fixtures),
    });
    let _ = E2E_STATE.set(state);
}

async fn bootstrap_fixtures(system: &Valence) -> anyhow::Result<FixtureIds> {
    let owner_ctx = system.with_actor(Actor::User {
        user_id: "owner".to_string(),
    });
    let peer_ctx = system.with_actor(Actor::User {
        user_id: "peer".to_string(),
    });

    let owner_tag = tag::create(
        TagCreateInput {
            name: "Seeded Office".into(),
            taxonomy: Some("spend".into()),
            description: Some("e2e fixture owned by owner".into()),
        },
        &owner_ctx,
    )
    .await?;

    let peer_tag = tag::create(
        TagCreateInput {
            name: "Peer Label".into(),
            taxonomy: Some("custom".into()),
            description: Some("e2e fixture owned by peer".into()),
        },
        &peer_ctx,
    )
    .await?;

    Ok(FixtureIds {
        tag_id: owner_tag.id,
        tag_name: "Seeded Office".into(),
        peer_tag_id: peer_tag.id,
        peer_tag_name: "Peer Label".into(),
    })
}

fn state() -> Arc<E2eState> {
    E2E_STATE
        .get()
        .expect("init_e2e_valence must run first")
        .clone()
}

pub fn e2e_router() -> Arc<DatabaseRouter> {
    Arc::clone(&state().router)
}

pub fn e2e_higgs_config() -> Arc<HiggsConfig> {
    Arc::clone(&state().higgs)
}

pub fn e2e_fixtures() -> FixtureIds {
    state().fixtures.lock().expect("fixtures").clone()
}

pub fn store_fixtures(fixtures: FixtureIds) {
    *state().fixtures.lock().expect("fixtures") = fixtures;
}

pub fn e2e_system_valence() -> Valence {
    Valence::builder()
        .database_router(e2e_router())
        .default_backend_key(state().default_backend_key.clone())
        .with_actor(Actor::System {
            operation: "e2e_seed".into(),
        })
        .build()
        .expect("system valence")
}

/// Mint a fresh owner-owned catalog row (for create/update isolation).
pub async fn seed_fresh_owner_tag(name: &str) -> anyhow::Result<FixtureIds> {
    let system = e2e_system_valence();
    let owner_ctx = system.with_actor(Actor::User {
        user_id: "owner".to_string(),
    });
    let created = tag::create(
        TagCreateInput {
            name: name.into(),
            taxonomy: Some("spend".into()),
            description: Some("fresh e2e seed".into()),
        },
        &owner_ctx,
    )
    .await?;
    let mut fixtures = e2e_fixtures();
    fixtures.tag_id = created.id;
    fixtures.tag_name = name.into();
    store_fixtures(fixtures.clone());
    Ok(fixtures)
}
