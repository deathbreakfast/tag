//! Protected `/tag` host: catalog create + `ManyToMany` attach recipe.
//!
//! Copy surfaces for product hosts: this package's `Cargo.toml` + `main.rs`,
//! plus the product-mount dependency / Leptos sketches in the host README.
//! Oneshot path `/tag` matches Orbital app id/path `tag` / `/tag`
//! (see JSON `inventory`).
//!
//! Tag owns the catalog; products declare `connections: [tags: { ManyToMany, … }]`.
//! This host creates catalog rows and an in-process edge list that mirrors that
//! composition without a product schema codegen step.
//!
//! ## When to use
//! Smoke tag catalog + protected `/tag` before mounting `TagRoutes`.
//!
//! ## Command
//! ```bash
//! export CARGO_BUILD_JOBS=1
//! export CARGO_TARGET_DIR=target-tag
//! cargo run -p protected-tag-host
//! ```
//!
//! ## Success
//! Stdout prints `protected_tag_host: OK — /tag deny/allow + catalog + ManyToMany`.
//!
//! ## Look next
//! Mount `<TagRoutes />` from `tag-app`; declare `ManyToMany` on product schemas.

#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]
#![allow(missing_docs)]

use std::sync::Arc;

use axum::body::Body;
use axum::extract::Extension;
use axum::http::{Request, StatusCode};
use axum::middleware::{from_fn, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use http_body_util::BodyExt;
use lepton_identity::generated::{User, UserStatus, UserUserType};
use tag::types::TagCreateInput;
use tag::{create, list};
use tower::ServiceExt;
use valence::{
    register_backend_logical_names, Actor, DatabaseBackend, DatabaseRouter, Model,
    RegisterBackendLogicalNamesOptions, SqliteBackend, Valence, SQLITE_ENGINE_ID,
};

#[derive(Clone)]
struct DemoSession {
    user_id: String,
}

#[derive(Clone)]
struct HostState {
    tags: Vec<(String, String)>,
    /// Demo product record id + attached tag ids (`ManyToMany` edge shape).
    note_id: String,
    attached_tag_ids: Vec<String>,
}

async fn require_session(req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    if req.extensions().get::<DemoSession>().is_some() {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

async fn inject_demo_session(mut req: Request<Body>, next: Next) -> Response {
    if let Some(user) = req
        .headers()
        .get("x-demo-user")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
    {
        req.extensions_mut().insert(DemoSession { user_id: user });
    }
    next.run(req).await
}

async fn setup_valence() -> Valence {
    valence::deletion::register_noop_deletion_dispatcher_for_tests();
    valence::clear_for_test();
    if std::env::var_os("VALENCE_OWNERSHIP_UNIFIED_FETCH").is_none() {
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
            operation: "protected-tag-host".into(),
        })
        .build()
        .expect("build valence")
}

async fn seed_user(id: &str, email: &str, valence: &Valence) {
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
    User::upsert_used(id, user, valence, valence::use_!(r"When **protected tag host** needs to persist work, we **save User** so the next step in that feature can continue with the latest values. People and services allowed for **protected tag host** use this data for that workflow—not as a general export of unrelated personal fields.")).await.expect("upsert user");
}

async fn bootstrap_tags() -> HostState {
    let system = setup_valence().await;
    seed_user("tag-owner", "owner@test.local", &system).await;
    let v = system.with_actor(Actor::User {
        user_id: "tag-owner".into(),
    });

    let office = create(
        TagCreateInput {
            name: "Office Supplies".into(),
            taxonomy: Some("spend".into()),
            description: Some("Catalog row".into()),
        },
        &v,
    )
    .await
    .expect("create office");
    let travel = create(
        TagCreateInput {
            name: "Travel".into(),
            taxonomy: Some("spend".into()),
            description: None,
        },
        &v,
    )
    .await
    .expect("create travel");

    let rows = list(&v, None, None).await.expect("list");
    assert!(rows.len() >= 2);

    // Product-side ManyToMany lives on the product schema (edge_table). Here we
    // record the edge shape a host would persist after `note.tags().relate(...)`.
    let note_id = "demo-note-1".to_string();
    let attached_tag_ids = vec![office.id.clone(), travel.id.clone()];
    let edge_table = "demo_note_tag";
    let _ = edge_table; // documented in JSON response

    HostState {
        tags: rows.into_iter().map(|r| (r.id, r.name)).collect(),
        note_id,
        attached_tag_ids,
    }
}

async fn tag_api(
    Extension(session): Extension<DemoSession>,
    Extension(state): Extension<HostState>,
) -> impl IntoResponse {
    Json(serde_json::json!({
        "path": "/tag",
        "user": session.user_id,
        "catalog": state.tags.iter().map(|(id, name)| serde_json::json!({
            "id": id,
            "name": name,
        })).collect::<Vec<_>>(),
        "many_to_many": {
            "product_model": "demo_note",
            "edge_table": "demo_note_tag",
            "note_id": state.note_id,
            "tag_ids": state.attached_tag_ids,
            "schema_fragment": "connections: [ tags: { table: \"tag\", cardinality: ManyToMany, edge_table: \"demo_note_tag\", model: \"tag::generated::Tag\" } ]",
        },
        // Matches tag-app `uf_app!` (not a Gauge permission manifest).
        "inventory": {
            "app_id": "tag",
            "route_path": "/tag",
            "auth_gate": "RequireAuthenticated",
        },
    }))
}

fn app(state: HostState) -> Router {
    Router::new()
        .route("/tag", get(tag_api))
        .route_layer(from_fn(require_session))
        .layer(Extension(state))
        .layer(from_fn(inject_demo_session))
}

#[tokio::main]
async fn main() {
    let state = bootstrap_tags().await;

    let denied = app(state.clone())
        .oneshot(
            Request::builder()
                .uri("/tag")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot")
        .status();
    assert_eq!(denied, StatusCode::UNAUTHORIZED);

    let response = app(state)
        .oneshot(
            Request::builder()
                .uri("/tag")
                .header("x-demo-user", "demo-ops")
                .body(Body::empty())
                .expect("req"),
        )
        .await
        .expect("oneshot");
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body")
        .to_bytes();
    let body: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(body["path"], "/tag");
    assert!(body["catalog"].as_array().expect("arr").len() >= 2);
    assert_eq!(
        body["many_to_many"]["tag_ids"]
            .as_array()
            .expect("ids")
            .len(),
        2
    );
    assert_eq!(body["inventory"]["app_id"], "tag");
    assert_eq!(body["inventory"]["route_path"], "/tag");
    assert_eq!(body["inventory"]["auth_gate"], "RequireAuthenticated");

    println!("protected_tag_host: OK — /tag deny/allow + catalog + ManyToMany");
}
