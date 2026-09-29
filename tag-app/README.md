# tag-app

Orbital admin UI for the shared tag catalog (`TagRoutes` at `/tag`).

## Host integration

Mount `<TagRoutes />` under the host `<Routes>`. Domain CRUD and history writers
live in the `tag` crate; this package wraps them for operators and exposes
`TagCatalogPicker` for other product forms. Shell chrome comes from
`uf-integrations` (same pattern as counter-app).

## Documentation

- Crate rustdoc: `cargo doc -p tag-app --features ssr --open` (Organized by
  task, Owns, Concern → API, Examples)
- Root [`README.md`](../README.md)

## Tests

`cargo test -p tag-app --features ssr --lib` covers error mapping, the
`search_tag_catalog` and `find_tag_by_name` server fns against in-memory
SQLite, and the picker's pure helpers (create offer, label cache, request
ordering). Behavioral contracts live in the sibling `tag` crate
(`product_surface`, `tag_crud_contract`, `tag_service_integration`,
`tag_unique_name`) and Playwright in `examples/tag-ui-e2e`. Picker and
create-dialog flows are exercised end to end by the Catalyst and Finance lab
hosts.
