pub mod protocol;
pub mod packet;
pub mod server;
pub mod connection;
pub mod protocol_tests;

#[cfg(feature = "opensrv")]
mod opensrv_shim;
#[cfg(feature = "opensrv")]
mod opensrv_server;

#[cfg(feature = "opensrv")]
pub use opensrv_server::MysqlServer;
#[cfg(not(feature = "opensrv"))]
pub use server::MysqlServer;

#[cfg(feature = "opensrv")]
pub use opensrv_shim::DorisShim;

pub use connection::MysqlConnection;
pub use protocol::*;
pub use packet::*;
