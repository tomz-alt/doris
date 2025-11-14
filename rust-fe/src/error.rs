use thiserror::Error;

#[derive(Error, Debug)]
pub enum DorisError {
    #[error("MySQL protocol error: {0}")]
    MysqlProtocol(String),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Query execution error: {0}")]
    QueryExecution(String),

    #[error("Backend communication error: {0}")]
    BackendCommunication(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Query queue full")]
    QueueFull,

    #[error("Connection closed")]
    ConnectionClosed,

    #[error("Invalid packet: {0}")]
    InvalidPacket(String),
}

pub type Result<T> = std::result::Result<T, DorisError>;
