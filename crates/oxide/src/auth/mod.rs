//! Twitch OAuth device authorization and token validation.

mod client;
mod store;

pub use client::AuthClient;
pub use store::{StoredTokens, TokenStore};

pub const REQUIRED_SCOPES: &[&str] = &[
    "user:read:chat",
    "user:write:chat",
    "moderator:manage:banned_users",
];
