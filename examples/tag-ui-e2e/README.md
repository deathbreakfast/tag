# tag-ui-e2e

Leptos lab host that mounts Tag catalog pages for Playwright. Lab-only:
insecure session cookies, `POST /api/test/seed-data`, harness auth (no lepton
sign-in). No Mailpit / OIDC / SMS.

## Run

From the repository root:

```bash
export CARGO_BUILD_JOBS=1
export CARGO_TARGET_DIR=target-tag
cd examples/tag-ui-e2e/end2end && npm ci && npx playwright install chromium && cd ../../..
cargo leptos end-to-end --project tag-ui-e2e
```

Host listens on `127.0.0.1:3160`. Do not Ctrl-C; the run exits when Playwright finishes.

The lab host mounts the same page components as `TagRoutes`, without `Lazy`
(wasm-split Lazy under `ParentRoute` panics on hydrate in the current Leptos pin).
Production hosts keep `TagRoutes` + `--split` when needed.

## Seed

`POST /api/test/seed-data` with JSON
`{ "auth": "owner" | "peer" | "unverified" | "anonymous", "seed_catalog": false }`.

When `seed_catalog` is true, mints a fresh owner-owned tag and returns its id/name
in `fixtures` (use for update/delete isolation).

## Scenario catalog (implemented)

Auth: `pw-tag-anon-gate-sad`, `pw-tag-unverified-gate-sad`

List: `pw-tag-list-load-happy`, `pw-tag-search-miss-sad`

Create: `pw-tag-create-happy`

Detail: `pw-tag-update-happy`, `pw-tag-delete-happy` (soft-delete: detail
becomes not-found / pending after delete), `pw-tag-not-found-sad`

History: `pw-tag-history-created-on-detail-happy`,
`pw-tag-history-after-update-happy`, `pw-tag-create-writes-history-happy`

Non-owner mutate deny stays covered by Layer 1 Valence integration
(`non_owner_cannot_update_other_users_tag_sad`); UI does not surface a distinct
peer-deny MessageBar on detail load for readable peer tags.
