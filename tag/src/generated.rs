#![allow(
    dead_code,
    unused_imports,
    missing_docs,
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::restriction
)]
//! Valence-codegen output for the tag catalog schemas (`build.rs` + `schemas/`).
//! Generated model types are not hand-documented; see `../schemas/*.rs` for the
//! source-of-truth field definitions.

#[cfg(feature = "ssr")]
use crate::privacy_policies::{TAG_DATA_OWNER, TAG_READ_ACTIVE_BOT};
#[cfg(feature = "ssr")]
use crate::side_effects::history_writer::TagHistoryWriter;
#[cfg(feature = "ssr")]
use valence::privacy_policies::common::{AUTHENTICATED, BLOCK_ALL, SYSTEM_ONLY};

#[cfg(feature = "ssr")]
include!(concat!(env!("OUT_DIR"), "/generated_models.rs"));
