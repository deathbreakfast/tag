# tag verification

Re-run after code or doc changes. This workspace is the Tag product
(`tag` catalog service + `tag-app` Leptos UI / `TagRoutes`). Layer 1 covers
the product-local catalog CRUD that backs `tag-app` server functions
(`create_tag`, `list_tags`, `get_tag`, `find_tag_by_name`, `update_tag`,
`delete_tag`), plus
sibling-source UI surface **contracts** for `tag-app` (source needles — not
runtime validation). Layer 2 is Playwright against the `tag-ui-e2e` lab host.
Valence / record-history own persistence primitives; this repo verifies the
tag catalog mapping layer and the operator-visible `/tag` workflows.

Prefer `cargo test -p tag --features ssr` for service suites and the
featureless `workspace_members` / `product_surface` binaries for UI contract
gates.

## Environment

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-tag
```

## Teaching host

Axum oneshot under [`examples/protected-tag-host`](../examples/protected-tag-host/).
Copy table + product mount sketches live in that host README. **Smoke / teaching
only** — not Playwright product E2E.

```bash
cargo check -p protected-tag-host
cargo run -p protected-tag-host
```

Success line: `protected_tag_host: OK — /tag deny/allow + catalog + ManyToMany`.

## Layer 1 — Unit + integration (CI)

### PR CI parity (`.github/workflows/ci.yml`)

Standalone checkout on `unified-field-dev/tag`. UF deps use git `branch = "main"`.
No monorepo sibling clones. `tag-app` hydrate and Playwright are **not** CI jobs.

| Job | Commands |
|-----|----------|
| **fmt** | `cargo fmt -p tag -p protected-tag-host -- --check` |
| **clippy** | `cargo clippy -p tag --all-targets --features ssr -- -D warnings` then `cargo clippy -p protected-tag-host --all-targets -- -D warnings` |
| **test** | `cargo test -p tag --test workspace_members --test product_surface`; domain `--features ssr` contract suites; `cargo check` + `cargo run -p protected-tag-host` |
| **docs** | `RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p tag --features ssr --no-deps` |

Toolchain: `nightly-2026-08-07` (pinned; floating tip breaks surrealdb/`diskann-wide`).

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-tag

cargo fmt -p tag -p protected-tag-host -- --check
cargo clippy -p tag --all-targets --features ssr -- -D warnings
cargo clippy -p protected-tag-host --all-targets -- -D warnings
cargo test -p tag --test workspace_members --test product_surface
cargo test -p tag --features ssr \
  --test tag_crud_contract \
  --test tag_service_integration \
  --test privacy_policy_integration \
  --test tag_unique_name
cargo check -p protected-tag-host
cargo run -p protected-tag-host
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p tag --features ssr --no-deps
```

`product_surface` is sibling-source UI **contracts** (file needles — not runtime
validation). Domain contract suites above are the behavioral merge gate.

Full package (includes featureless surface gates + SSR integ):

```bash
cargo test -p tag --features ssr
```

Full workspace (includes `tag-app` UI). May fail when the Orbital /
`uf-product` UI graph is broken upstream — that is a pre-existing host product
UI compile issue, not a tag catalog contract gap:

```bash
cargo clippy --workspace --all-targets --features ssr -- -D warnings
cargo test --workspace --features ssr
```

`tag-app` unit tests (server fns against in-memory SQLite, picker helpers):

```bash
cargo test -p tag-app --features ssr --lib
```

`tag-app` (Leptos UI + Higgs `#[server]` wrappers) may fail to compile when the
Orbital / `uf-product` graph drifts upstream. Prefer the `tag` crate for CI
contract gates; treat UI-crate compile failures as a separate host product
issue, not a catalog-domain gap. Surface needles for routes, nav testids,
`RequireAuthenticated`, and page bindings live in `product_surface` (smoke).
Runtime UI validation is Layer 2.

## Layer 2 — E2E (Playwright)

Required for operator-visible `/tag` workflows. Lab host
[`examples/tag-ui-e2e`](../examples/tag-ui-e2e/) mounts eager Tag pages
(same components as `TagRoutes`, without `Lazy`), harness session seed, and
Chromium Playwright. No Mailpit / OIDC / SMS. Scenario catalog:
[`examples/tag-ui-e2e/README.md`](../examples/tag-ui-e2e/README.md).

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-tag
cd examples/tag-ui-e2e/end2end && npm ci && npx playwright install chromium && cd ../../..
cargo leptos end-to-end --project tag-ui-e2e
```

Host listens on `127.0.0.1:3160`. Do **not** Ctrl-C; the run exits when
Playwright finishes.

Validating scenarios (happy + sad):

- `pw-tag-anon-gate-sad` / `pw-tag-unverified-gate-sad`
- `pw-tag-list-load-happy` / `pw-tag-search-miss-sad`
- `pw-tag-create-happy`
- `pw-tag-update-happy` / `pw-tag-delete-happy` (soft-delete: detail no longer editable) / `pw-tag-not-found-sad`
- `pw-tag-history-created-on-detail-happy` / `pw-tag-history-after-update-happy` / `pw-tag-create-writes-history-happy`

Non-owner mutate containment remains Layer 1
(`non_owner_cannot_update_other_users_tag_sad` /
`non_owner_cannot_delete_other_users_tag_sad`).

## Layer 3 — Cloud / performance

**Waived.** L3-local product; no cloud resources or Criterion benches.
Correctness is in-process against an embedded SQLite `:memory:` Valence (aligned
with tag schema `SQLITE_ENGINE_ID`) for Layer 1, and the lab host for Layer 2.
Defer any soak unless a shared hot path changes. L5 IsolatedLab
(Spectra→…→Photon) is out of scope for this product gate.

## Rustdoc

Workspace `Cargo.toml` allows `broken_intra_doc_links` by default. Honest local
deny for the domain crate:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-tag
RUSTDOCFLAGS="-D rustdoc::broken-intra-doc-links" cargo doc -p tag --features ssr --no-deps
```

`tag-app` rustdoc with deny flags is pin-dependent on Orbital / `uf-product`
(and may hit turf/cssparser skew). Prefer the `tag` gate above for docs CI
signal. `tag-app` still uses `#![allow(missing_docs)]` on macro-heavy surfaces.

### leptos-lints (local; not a CI hard gate yet)

Needs `cargo-dylint` / `dylint-link` 6.0.1 and toolchain `nightly-2025-05-14`
(leptos-lints v0.1.2 pin). Workspace `Cargo.toml` denies the leptos_* lint names
and pins dylint metadata. CI does not hard-gate this yet: `tag-app` hydrate
still fails to compile under the pinned dylint nightly when the Orbital /
`uf-product` graph drifts (same class of issue as optional UI checks
elsewhere). Re-enable a CI job when host Orbital pins match.

```bash
# cargo install cargo-dylint --locked --version 6.0.1
# cargo install dylint-link --locked --version 6.0.1
# rustup toolchain install nightly-2025-05-14 --component rustc-dev,llvm-tools-preview

export CARGO_RESOLVER_INCOMPATIBLE_RUST_VERSIONS=fallback
export RUSTFLAGS="-D warnings -Zcrate-attr=feature(stdarch_x86_avx512)"
cargo dylint --all -p tag-app --no-deps -- --features hydrate
```

## Notes

- Prefer `cargo test -p tag --features ssr` for backend contract CI when the UI
  dependency graph fails to compile — report that separately from tag contract
  results.
- Tests may `unwrap`/`expect`; production server fns map failures to
  `ServerFnError` (no ordinary-path unwrap).
- Sad-path assertions check message content, typed `TagError` variants where
  practical, or `None` / empty — stronger than `is_err()` alone.
- Happy-path tests are named `*_happy_path` (Layer 1) or `pw-*-happy` (Layer 2)
  so audits detect them.
- `product_surface` `*_sad_path` names are **source-needle contracts**, not
  runtime sad paths.
