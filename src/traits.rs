//! Salish Message Traits

use std::any::Any;

/// Sealed Message traits
pub(crate) mod internal {
    use std::any::TypeId;

    pub trait SalishMessageInternal: super::SalishMessage {
        /// Downcast to the inner concrete type T.
        /// Returns None if the message could not be downcast
        #[allow(dead_code)]
        fn inner<'b, T>(&'b self) -> Option<&'b T>
        where
            T: 'static,
        {
            let message = self.payload().as_any().downcast_ref::<T>();

            #[cfg(debug_assertions)]
            if message.is_none() {
                panic!("Failed to downcast message");
            }

            message
        }

        fn into_inner<T>(self) -> Option<T>
        where
            T: 'static,
            Self: Sized,
        {
            let payload = self.payload().clone_any();
            match payload.downcast::<T>() {
                Ok(inner) => Some(*inner),
                Err(_) => {
                    panic!("Failed to downcast message")
                }
            }
        }

        /// Get the [`TypeId`] of the message payload
        fn payload_type(&self) -> TypeId {
            self.payload().as_any().type_id()
        }
    }
}

/// Salish message trait, implemented by message containers for routing and unwrapping inner message types
pub trait SalishMessage {
    /// The [`Endpoint`] type that messages can be routed to
    type Endpoint: EndpointAddress;

    fn payload(&self) -> &dyn Payload;

    /*
    /// Implementations of [`SalishMessage`] must provide a method that returns a `&dyn Any`
    fn as_any(&self) -> &dyn Any;
    */
}

/// Implement sealed trait [`SalishMessageInternal`] on anything implementing [`SalishMessage`]
impl<T> internal::SalishMessageInternal for T where T: SalishMessage {}

/// Message Payload
pub trait Payload: Any + Send + Sync + std::fmt::Debug + 'static {
    fn clone_payload(&self) -> Box<dyn Payload> {
        panic!("Payload type does not implement Clone")
    }

    fn clone_any(&self) -> Box<dyn Any> {
        panic!("Payload type does not implement Clone")
    }

    fn as_any(&self) -> &dyn Any;
}

/// Allow Box<dyn Payload> to be cloned with `.clone_box` if the inner type
/// implements Clone
impl<T> Payload for T
where
    T: Any + Clone + Send + Sync + std::fmt::Debug + 'static,
{
    fn clone_payload(&self) -> Box<dyn Payload> {
        Box::new(self.clone())
    }

    fn clone_any(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Message Endpoint
pub trait EndpointAddress: std::fmt::Debug + Send + Sync {
    type Addr;
    fn addr(&self) -> Self::Addr;
}
