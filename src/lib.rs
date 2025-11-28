// ABOUTME: Main library file for micropub CLI
// ABOUTME: Exports all public modules and types

pub mod auth;
pub mod client;
pub mod config;
pub mod draft;
pub mod media;
// pub mod mcp;  // Blocked: rmcp macro issues persist in v0.8 and v0.9
pub mod operations;
pub mod publish;
pub mod tui;

pub use anyhow::{Error, Result};
