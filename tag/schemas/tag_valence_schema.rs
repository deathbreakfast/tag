#[allow(unused_imports)]
use crate::privacy_policies::{TAG_DATA_OWNER, TAG_READ_ACTIVE_BOT};
use valence::prelude::*;
use valence::privacy_policies::common::{AUTHENTICATED, SYSTEM_ONLY};

valence_schema! {
    Tag {
        repository: "https://github.com/unified-field-dev/tag",
        table: "tag",
        version: "0.2.0",
        database: crate::embedded_surreal::DEFAULT_STORAGE,
        description: "Shared tag catalog row (name, taxonomy, description). Any signed-in user, or an active bot acting for its owner, may read; any signed-in user may create; only the owner may update or delete.",

        traits: [HistorySource],

        policies: {
            // Shared catalog: any signed-in user may read, and so may an active
            // bot acting for its owner. Other service users are denied, and
            // bots get no create, update or delete.
            read: {
                allow: [AUTHENTICATED, TAG_READ_ACTIVE_BOT, SYSTEM_ONLY],
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
            // `normalize_name_key(name)`; the unique index makes names unique
            // ignoring case and surrounding whitespace.
            name_key: {
                r#type: FieldType::String,
                required: true,
                unique: true,
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
