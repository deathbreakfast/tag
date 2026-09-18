#[allow(unused_imports)]
use crate::privacy_policies::TAG_DATA_OWNER;
use valence::prelude::*;
use valence::privacy_policies::common::{AUTHENTICATED, SYSTEM_ONLY};

valence_schema! {
    Tag {
        repository: "https://github.com/unified-field-dev/tag",
        table: "tag",
        version: "0.1.0",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "Shared tag catalog row (name, taxonomy, description)",

        traits: [HistorySource],

        policies: {
            // Shared/public catalog: any authenticated actor may read; mutations stay owner-scoped.
            read: {
                allow: [AUTHENTICATED, SYSTEM_ONLY],
            },
            create: {
                allow: [AUTHENTICATED, SYSTEM_ONLY],
            },
            update: {
                allow: [TAG_DATA_OWNER, SYSTEM_ONLY],
            },
            delete: {
                allow: [TAG_DATA_OWNER, SYSTEM_ONLY],
            },
        },

        fields: [
            id: {
                r#type: FieldType::String,
                primary_key: true,
                required: true,
            },
            name: {
                r#type: FieldType::String,
                required: true,
            },
            taxonomy: {
                r#type: FieldType::String,
                required: false,
            },
            description: {
                r#type: FieldType::String,
                required: false,
            },
            created_at: {
                r#type: FieldType::DateTime,
                required: true,
            },
            updated_at: {
                r#type: FieldType::DateTime,
                required: true,
            },
        ],

    }
}
