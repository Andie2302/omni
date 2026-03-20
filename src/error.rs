use thiserror::Error;

#[derive(Debug, Error)]
pub enum OmniError {
    #[error("Unknown: {0}")]
    Unknown(#[from] std::io::Error),
    #[error("CreateDirectory: {0}")]
    CreateDirectory(std::io::Error),
    #[error("CreateFile: {0}")]
    CreateFile(std::io::Error),
}

