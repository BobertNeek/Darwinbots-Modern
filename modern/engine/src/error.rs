use thiserror::Error;

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("legacy DNA parse error on line {line}: {message}")]
    DnaParse { line: usize, message: String },
    #[error("organism identifier refers to a removed or replaced organism")]
    StaleOrganismId,
    #[error("the configured organism capacity has been reached")]
    CapacityReached,
    #[error("GPU backend is unavailable: {0}")]
    GpuUnavailable(String),
    #[error("GPU operation failed: {0}")]
    Gpu(String),
    #[error("save data is invalid: {0}")]
    InvalidSave(String),
    #[error("simulation invariant failed: {0}")]
    Invariant(String),
    #[error("serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}
