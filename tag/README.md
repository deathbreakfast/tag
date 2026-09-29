# Tag

Shared label catalog with Valence connections (no assignment table).

Tag stores catalog rows (`name`, `taxonomy`, `owner`, timestamps). Names are
unique ignoring case and surrounding spaces, enforced by a unique index on
`name_key`. Product
records link tags through **Valence `connections` (ManyToMany)**. Definition
edits append `tag_history` rows; detail pages embed
`<HistoryTimeline source_id />` from
[record-history](https://github.com/unified-field-dev/record-history).

## Responsibilities

| Crate | Role |
|-------|------|
| `tag` | `Tag` schema, CRUD, optional `tag_history` |
| `tag-app` | `/tag` UI, `TagCatalogPicker`, timeline on detail |

## How products attach tags

```rust
// On a product record — illustrative
connections: [
    tags: {
        table: "tag",
        cardinality: ManyToMany,
        edge_table: "finance_transaction_tag",
        model: "tag::generated::Tag",
    },
],
```

Use generated connection APIs to add or remove tags. Connection changes can
append record-history rows (`field_name: "tags"`).

## Documentation

- Crate rustdoc: `cargo doc -p tag --features ssr --open` (Organized by task,
  Owns, Concern → API, Examples)
- Root [`README.md`](../README.md) and [`docs/VERIFICATION.md`](../docs/VERIFICATION.md)
