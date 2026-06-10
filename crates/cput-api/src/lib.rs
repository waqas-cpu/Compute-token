//! HTTP API for the Compute Token dashboard — reads pipeline queue + relayer store.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod handlers;
pub mod state;
mod types;

pub use handlers::router;
pub use state::{ApiConfig, ApiState};
