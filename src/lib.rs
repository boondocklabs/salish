//! Salish Application Messaging

pub mod endpoint;
pub mod filter;
pub mod handler;
pub mod message;
pub mod policy;
pub mod router;
pub mod traits;

pub use message::Message;
pub use traits::EndpointAddress;

#[cfg(test)]
mod test;
