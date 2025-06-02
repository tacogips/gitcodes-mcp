//! GitDB MCP Library
//!
//! This library provides Model Context Protocol (MCP) tools.
//!
//! ## Note
//!
//! This library is currently a placeholder. The mock implementations have been removed
//! to fix compilation errors caused by references to non-existent modules.
//!
//! ## Usage
//!
//! This library can be used in several ways:
//! - As an MCP server (HTTP/SSE mode)
//! - As an MCP server (STDIN/STDOUT mode)
//! - Directly as a Rust library
//!
//! See the README.md file for more usage examples.

pub mod git;
pub mod services;
pub mod tools;
pub mod transport;
