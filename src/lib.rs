//! Salish Application Messaging

pub mod endpoint;
pub mod handler;
pub mod message;
pub mod policy;
pub mod router;
pub mod traits;

pub use message::Message;

#[cfg(test)]
mod test;
