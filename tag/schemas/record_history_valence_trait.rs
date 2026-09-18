use valence::prelude::*;
use valence::privacy_policies::common::{BLOCK_ALL, SYSTEM_ONLY};

valence_trait_schema! {
    RecordHistory {
        repository: "https://github.com/unified-field-dev/tag",
        policies: {
            // Inherit parent Read via the `source` edge (Valence defer_to_edge).
            // Create/delete stay System-only; TagHistory overrides for session append.
            read: { defer_to_edge: "source" },
            create: { allow: [SYSTEM_ONLY] },
            update: { always_block: [BLOCK_ALL] },
            delete: { allow: [SYSTEM_ONLY] },
        },
        fields: [
            id: { r#type: FieldType::String, primary_key: true, required: true },
            source: {
                r#type: FieldType::Record("e2e_history_source_a"),
                required: true,
            },
            field_name: { r#type: FieldType::String, required: true },
            old_value: { r#type: FieldType::String, required: true },
            new_value: { r#type: FieldType::String, required: true },
            changed_at: { r#type: FieldType::DateTime, required: true },
            actor: {
                r#type: FieldType::Record("user"),
                required: false,
            },
        ],
        connections: [
            source: {
                table: "trait:HistorySource",
                cardinality: HasOne,
                required: true,
                on_delete: Restrict,
                target_trait: "HistorySource",
            },
            actor: {
                table: "user",
                cardinality: HasOne,
                required: false,
                on_delete: SetNull,
                // tag depends on lepton-identity (not full lepton).
                model: "lepton_identity::generated::User",
            },
        ],
    }
}
