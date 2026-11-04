//! llmo — Local LLM Memory & Runtime Orchestrator.
//!
//! A single local daemon that sits between your apps and your local model
//! servers. It exposes one OpenAI-compatible endpoint, routes each request to
//! the right runtime (launching it on demand within a RAM/VRAM budget), and
//! gives every conversation persistent, private, searchable long-term memory
//! stored in SQLite on your own disk.

pub mod auth;
pub mod chat;
pub mod config;
pub mod error;
pub mod hardware;
pub mod memory;
pub mod metrics;
pub mod planner;
pub mod runtime;
pub mod server;
