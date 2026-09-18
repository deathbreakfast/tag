use valence::prelude::*;

valence_schema! {
    TagHistory {
        repository: "https://github.com/unified-field-dev/tag",
        table: "tag_history",
        version: "0.1.3",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "Append-only edit history for tag catalog rows",

        traits: [RecordHistory],

        policies: {
            // Inherit parent Tag access via source (Create→parent Update).
            read: { defer_to_edge: "source" },
            create: { defer_to_edge: "source" },
            update: { defer_to_edge: "source" },
            delete: { defer_to_edge: "source" },
        },

        fields: [
            subject_display_name: {
                r#type: FieldType::String,
                required: false,
            },
        ],
    }
}
