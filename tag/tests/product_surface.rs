//! Product surface **contracts** for tag-app (sibling crate) — source needles.
//!
//! These are file-string gates, not runtime happy/sad validation. Playwright
//! Layer 2 (`tag-ui-e2e`) covers operator-visible behavior. Lives under `tag`
//! so CI can gate route/testid/auth needles without compiling Orbital/turf UI
//! when host pins churn. Pattern matches lepton-uf-app
//! `lepton-shell/tests/product_surface.rs` and gauge `gauge/tests/product_surface.rs`.
//!
//! Names ending in `_sad_path` mean "detect missing needles," not runtime
//! failure classification.
use std::fs;
use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn read_app(rel: &str) -> String {
    let path = workspace_root().join("tag-app").join("src").join(rel);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

#[test]
fn tag_routes_mount_happy_path() {
    let lib = read_app("lib.rs");
    for needle in [
        r#"path!("tag")"#,
        r#"path!("")"#,
        r#"path!("create")"#,
        r#"path!(":id")"#,
        "TagVerifiedGuardRouteView",
        "id: \"tag\"",
        "route_path: \"/tag\"",
    ] {
        assert!(
            lib.contains(needle),
            "TagRoutes / uf_app missing `{needle}`"
        );
    }
}

#[test]
fn tag_routes_drop_leaf_sad_path() {
    let lib = read_app("lib.rs");
    for needle in [r#"path!("create")"#, r#"path!(":id")"#] {
        assert!(
            lib.contains(needle),
            "removing `{needle}` drops a Tags funnel entry"
        );
    }
    assert!(
        !lib.contains("unimplemented!"),
        "TagRoutes must not ship unimplemented placeholders"
    );
}

#[test]
fn uf_app_wrong_id_sad_path() {
    let lib = read_app("lib.rs");
    assert!(
        lib.contains("id: \"tag\""),
        "wrong uf_app id breaks Orbital host registration"
    );
    assert!(
        lib.contains("route_path: \"/tag\""),
        "uf_app route_path must stay /tag"
    );
}

#[test]
fn layout_uses_platform_shell_happy_path() {
    let layout = read_app("layout.rs");
    for needle in [
        "uf_integrations",
        "UnifiedFieldAppBar",
        "UnifiedFieldShellLayout",
        "tag-app-root",
        "nav-tags",
        "Outlet",
        "AppBarUserMenu",
    ] {
        assert!(
            layout.contains(needle),
            "TagAppLayout missing contract `{needle}`"
        );
    }
    assert!(
        !std::path::Path::new(&format!("{}/tag-app/src/shell", workspace_root().display()))
            .exists(),
        "tag-app must not vendor a local shell; use uf-integrations"
    );
}

#[test]
fn layout_delegates_notification_bell_to_platform_happy_path() {
    let layout = read_app("layout.rs");
    assert!(
        layout.contains("uf_integrations") && layout.contains("UnifiedFieldAppBar"),
        "TagAppLayout must use uf_integrations::UnifiedFieldAppBar (hosts HostNotificationBell)"
    );
    let cargo = fs::read_to_string(workspace_root().join("tag-app/Cargo.toml"))
        .expect("tag-app Cargo.toml");
    assert!(
        cargo.contains("uf-integrations"),
        "tag-app must depend on uf-integrations for platform shell chrome"
    );
}

#[test]
fn layout_missing_platform_app_bar_sad_path() {
    let layout = read_app("layout.rs");
    assert!(
        layout.contains("UnifiedFieldAppBar"),
        "dropping UnifiedFieldAppBar removes branding, search, and HostNotificationBell"
    );
}

#[test]
fn verified_guard_auth_happy_path() {
    let lazy = read_app("lazy_routes.rs");
    for needle in [
        "RequireAuthenticated",
        "requires_email_verification=true",
        "TagAppLayout",
        "TagListPage",
        "TagCreatePage",
        "TagDetailPage",
    ] {
        assert!(
            lazy.contains(needle),
            "TagVerifiedGuardRouteView / lazy routes missing `{needle}`"
        );
    }
}

#[test]
fn verified_guard_drop_auth_sad_path() {
    let lazy = read_app("lazy_routes.rs");
    assert!(
        lazy.contains("RequireAuthenticated") && lazy.contains("TagAppLayout"),
        "removing RequireAuthenticated opens /tag pages to anonymous sessions"
    );
    assert!(
        lazy.contains("requires_email_verification=true"),
        "guard must keep email-verification gate on the tag outlet"
    );
}

#[test]
fn layout_missing_nav_sad_path() {
    let layout = read_app("layout.rs");
    assert!(
        layout.contains("nav-tags"),
        "dropping nav-tags breaks operator left-nav contract"
    );
    assert!(
        layout.contains("data-testid=\"tag-app-root\""),
        "dropping tag-app-root breaks host / future Playwright parity"
    );
}

#[test]
fn server_require_session_happy_path() {
    let server = read_app("server/mod.rs");
    assert!(
        server.contains("fn require_session")
            && server.contains("TagServerError::NotAuthenticated")
            && server.contains("session_user_id()"),
        "server must fail closed without a session"
    );
    let errors = read_app("server/error.rs");
    assert!(
        errors.contains("Authentication required") && errors.contains("into_server_error"),
        "TagServerError must map auth failures to a client-visible message"
    );
    for call_site in [
        "list_tags",
        "get_tag",
        "create_tag",
        "update_tag",
        "delete_tag",
    ] {
        assert!(server.contains(call_site), "server missing `{call_site}`");
    }
    let search = read_app("server/tag_catalog_search.rs");
    assert!(
        search.contains("search_tag_catalog") && search.contains("require_session"),
        "search_tag_catalog must require a session"
    );
}

#[test]
fn server_drop_require_session_on_list_sad_path() {
    let server = read_app("server/mod.rs");
    let start = server.find("pub async fn list_tags").expect("list_tags");
    let body = &server[start..start + 450.min(server.len() - start)];
    assert!(
        body.contains("require_session(&ctx)"),
        "list_tags must call require_session before service"
    );
}

#[test]
fn server_drop_require_session_on_create_sad_path() {
    let server = read_app("server/mod.rs");
    let start = server.find("pub async fn create_tag").expect("create_tag");
    let body = &server[start..start + 450.min(server.len() - start)];
    assert!(
        body.contains("require_session(&ctx)"),
        "create_tag must call require_session before service"
    );
}

#[test]
fn pages_testid_and_bindings_happy_path() {
    let list = format!(
        "{}\n{}",
        read_app("pages/list.rs"),
        read_app("components/tag_list_results.rs")
    );
    for needle in [
        "tag-list-page",
        "list_tags",
        "tag-create-button",
        "tag-list-filtered-empty",
    ] {
        assert!(list.contains(needle), "TagListPage missing `{needle}`");
    }

    let create = format!(
        "{}\n{}",
        read_app("pages/create.rs"),
        read_app("components/tag_create_dialog.rs")
    );
    for needle in [
        "tag-create-page",
        "create_tag",
        "TagCreateFields",
        "tag-create-name",
        "tag-create-submit",
    ] {
        assert!(create.contains(needle), "TagCreatePage missing `{needle}`");
    }

    let detail = read_app("pages/detail.rs");
    for needle in [
        "tag-detail-page",
        "get_tag",
        "update_tag",
        "delete_tag",
        "tag-detail-save",
        "tag-detail-delete",
        "HistoryTimeline",
    ] {
        assert!(detail.contains(needle), "TagDetailPage missing `{needle}`");
    }
}

#[test]
fn pages_drop_list_testid_sad_path() {
    let list = read_app("pages/list.rs");
    assert!(
        list.contains("data_testid=\"tag-list-page\""),
        "dropping tag-list-page breaks host / future Playwright parity"
    );
    let create = read_app("pages/create.rs");
    assert!(
        create.contains("data_testid=\"tag-create-page\""),
        "dropping tag-create-page breaks host / future Playwright parity"
    );
    let detail = read_app("pages/detail.rs");
    assert!(
        detail.contains("data_testid=\"tag-detail-page\""),
        "dropping tag-detail-page breaks host / future Playwright parity"
    );
}

#[test]
fn detail_missing_delete_sad_path() {
    let detail = read_app("pages/detail.rs");
    assert!(
        detail.contains("delete_tag"),
        "detail page must bind delete_tag for catalog removal"
    );
    assert!(
        detail.contains("HistoryTimeline"),
        "detail page must embed HistoryTimeline for tag_history"
    );
    assert!(
        !detail.contains("unimplemented!"),
        "detail page must not ship unimplemented placeholders"
    );
}

#[test]
fn catalog_picker_search_binding_happy_path() {
    let picker = read_app("components/tag_catalog_picker.rs");
    for needle in [
        "tag-catalog-picker",
        "search_tag_catalog",
        "TagSearchSourceId::Catalog",
        "tag-catalog-picker-manage",
        "on_search",
        "tag-catalog-picker-create-option",
        "TagCreateDialog",
    ] {
        assert!(
            picker.contains(needle),
            "TagCatalogPicker missing `{needle}`"
        );
    }
    assert!(
        !picker.contains("tag-catalog-picker-search"),
        "picker searches from the chip input, not a second search box"
    );
}

#[test]
fn search_tag_catalog_clamps_limit_happy_path() {
    let search = read_app("server/tag_catalog_search.rs");
    assert!(
        search.contains("SEARCH_TAG_CATALOG_MAX")
            && search.contains("clamp(1, SEARCH_TAG_CATALOG_MAX)")
            && search.contains("query_many"),
        "search_tag_catalog must clamp limit and query through the registry"
    );
    assert!(
        search.contains("InvalidSource") || search.contains("unsupported search source"),
        "search_tag_catalog must validate source_keys instead of discarding them"
    );
    assert!(
        !search.contains("let _ = &source_keys"),
        "source_keys must not be discarded unused"
    );
}

#[test]
fn catalog_picker_drop_testid_sad_path() {
    let picker = read_app("components/tag_catalog_picker.rs");
    assert!(
        picker.contains("data-testid=\"tag-catalog-picker\""),
        "dropping tag-catalog-picker breaks embedder / future Playwright parity"
    );
    assert!(
        picker.contains("search_tag_catalog"),
        "picker must call search_tag_catalog for catalog options"
    );
}

#[test]
fn protected_tag_host_matches_uf_app_happy_path() {
    let host = fs::read_to_string(workspace_root().join("examples/protected-tag-host/src/main.rs"))
        .expect("protected-tag-host main.rs");
    for needle in [
        "\"app_id\": \"tag\"",
        "\"route_path\": \"/tag\"",
        "\"auth_gate\": \"RequireAuthenticated\"",
        "TagCreateInput",
        "many_to_many",
        "demo_note_tag",
    ] {
        assert!(
            host.contains(needle),
            "protected-tag-host missing contract `{needle}`"
        );
    }
    let lib = read_app("lib.rs");
    assert!(
        lib.contains("id: \"tag\"") && lib.contains("route_path: \"/tag\""),
        "host inventory must stay aligned with uf_app!"
    );
    let lazy = read_app("lazy_routes.rs");
    assert!(
        lazy.contains("RequireAuthenticated"),
        "host auth_gate must stay aligned with TagRoutes guard"
    );
}
