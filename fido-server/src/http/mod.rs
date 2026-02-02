//! HTTP helpers and extractors for request metadata and auth.

pub mod auth;
pub mod headers;

pub use auth::{AuthenticatedUser, OptionalUser};
pub use headers::{extract_client_ip, extract_user_agent};
