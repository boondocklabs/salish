//! Salish Message Traits

use std::any::Any;

/// Sealed Message traits
pub(crate) mod internal {
    use std::any::TypeId;

    pub trait SalishMessageInternal: super::SalishMessage {
        /// Downcast to the inner concrete type T.
        /// Returns None if the message could not be downcast
        #[allow(dead_code)]
        fn inner<T>(&self) -> Option<&T>
        where
            T: 'static,
        {
            let message = match self.payload() {
                super::MessagePayload::Unicast(unicast_payload) => {
                    unicast_payload.as_ref().as_any().downcast_ref::<T>()
                }
                super::MessagePayload::Broadcast(broadcast_payload) => {
                    broadcast_payload.as_ref().as_any().downcast_ref::<T>()
                }
            };

            #[cfg(debug_assertions)]
            if message.is_none() {
                panic!("Failed to downcast message");
            }

            message
        }

        /// Consume and downcast the message
        fn into_inner<T>(self) -> Option<T>
        where
            T: 'static,
            Self: Sized,
        {
            let payload = self.to_payload();

            let payload = match payload {
                super::MessagePayload::Unicast(unicast_payload) => {
                    unicast_payload.into_any().downcast::<T>()
                }
                super::MessagePayload::Broadcast(broadcast_payload) => {
                    broadcast_payload.into_any().downcast::<T>()
                }
            };

            match payload {
                Ok(payload) => Some(*payload),
                Err(_) => {
                    panic!("Failed to downcast owned payload")
                }
            }
        }

        /// Get the [`TypeId`] of the message payload
        fn payload_type(&self) -> TypeId {
            match self.payload() {
                super::MessagePayload::Unicast(payload) => (**payload).as_any().type_id(),
                super::MessagePayload::Broadcast(payload) => (**payload).as_any().type_id(),
            }
        }
    }
}

/// Salish message trait, implemented by message containers for routing and unwrapping inner message types
pub trait SalishMessage: 'static {
    /// The [`Endpoint`] type that messages can be routed to
    type Endpoint: EndpointAddress;

    /// Return a reference to the [`MessagePayload`]
    fn payload<'b>(&'b self) -> &'b MessagePayload;

    fn to_payload(self) -> MessagePayload;
}

/// Implement sealed trait [`SalishMessageInternal`] on anything implementing [`SalishMessage`]
impl<T> internal::SalishMessageInternal for T where T: SalishMessage {}

/// Message Endpoint Address
pub trait EndpointAddress: std::fmt::Debug {
    type Addr;
    fn addr(&self) -> Self::Addr;
}

/// A broadcastable message payload
pub trait BroadcastPayload: Payload {
    /// Clone the payload into a `Box<dyn Any>`
    fn clone_payload(&self) -> Box<dyn BroadcastPayload>;
    fn into_payload(self) -> MessagePayload;
}

/// A unicast payload that does not implement `Clone`
pub trait UnicastPayload: Payload {
    fn into_payload(self) -> MessagePayload;
}

pub trait Payload: std::fmt::Debug + Send + Sync {
    fn as_any(&self) -> &dyn Any;
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

/// Implement [`BroadcastPayload`] for any type implementing
/// `Any + Clone + Send + Sync + Debug`
impl<T> BroadcastPayload for T
where
    T: Any + Sized + Clone + std::fmt::Debug + Send + Sync,
{
    fn clone_payload(&self) -> Box<dyn BroadcastPayload> {
        //MessagePayload::Broadcast(Box::new((*self).clone()))
        Box::new(self.clone())
    }

    fn into_payload(self) -> MessagePayload {
        MessagePayload::Broadcast(Box::new(self))
    }
}

/// Implement [`UnicastPayload`] for any type implementing
/// `Any + Debug`
impl<T> UnicastPayload for T
where
    T: Any + Sized + std::fmt::Debug + Send + Sync,
{
    fn into_payload(self) -> MessagePayload {
        MessagePayload::Unicast(Box::new(self))
    }
}

/// Implement [`Payload`] for any type implementing
/// `Any + Debug`
impl<T> Payload for T
where
    T: Any + std::fmt::Debug + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        Box::new(*self)
    }
}

#[derive(Debug)]
pub enum MessagePayload {
    Unicast(Box<dyn UnicastPayload>),
    Broadcast(Box<dyn BroadcastPayload>),
}

impl MessagePayload {
    pub fn as_any(&self) -> &dyn Any {
        match self {
            MessagePayload::Unicast(unicast_payload) => unicast_payload.as_any(),
            MessagePayload::Broadcast(broadcast_payload) => broadcast_payload.as_any(),
        }
    }
}

impl Clone for MessagePayload {
    fn clone(&self) -> Self {
        match self {
            MessagePayload::Unicast(_) => {
                panic!("Cannot clone Unicast payload");
            }
            MessagePayload::Broadcast(payload) => {
                MessagePayload::Broadcast((*payload).clone_payload())
            }
        }
    }
}
