//! TagRoutes Playwright host.
#![allow(missing_docs)]

mod app;
#[cfg(feature = "ssr")]
mod e2e_valence;
mod gate_demos;
mod harness_auth_menu;
mod picker_demo;
#[cfg(feature = "ssr")]
pub mod seed;
mod tag_routes_eager;

pub use app::{shell, App};
#[cfg(feature = "ssr")]
pub use e2e_valence::{e2e_higgs_config, e2e_router, init_e2e_valence};
#[cfg(feature = "ssr")]
pub use gate_demos::inject_e2e_session_snapshot;
