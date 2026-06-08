use thiserror::Error;

/// Domain-specific errors for krebslog.
/// Top-level run uses anyhow for convenience at the binary boundary.
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum KrebslogError {
    #[error("unrecognized date format: '{0}'. Use today, yesterday, 2026-05-20, last 7 days, last monday, etc.")]
    InvalidDate(String),

    #[error("unknown source: '{0}'. Supported: nutlog, repslog, bodylog")]
    UnknownSource(String),

    #[error("unknown entity for source {src}: '{entity}'")]
    UnknownEntity { src: String, entity: String },

    #[error("failed to execute external tool '{bin}': {reason}")]
    ExternalTool { bin: String, reason: String },

    #[error("external tool '{bin}' returned non-zero: {stderr}")]
    ExternalToolFailed { bin: String, stderr: String },

    #[error("failed to parse JSON from {bin}: {reason}")]
    JsonParse { bin: String, reason: String },

    #[error("cache disabled (--no-cache) but a cache operation was requested")]
    CacheDisabled,

    #[error("database error: {0}")]
    Database(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, KrebslogError>;

// Note: conversion to anyhow::Error happens automatically via the std::error::Error impl
// provided by thiserror (anyhow has a blanket From<E: std::error::Error + Send + Sync + 'static>).
