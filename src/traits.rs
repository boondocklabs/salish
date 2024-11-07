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
            let message = self.as_any().downcast_ref::<T>();

            #[cfg(debug_assertions)]
            if message.is_none() {
                panic!("Failed to downcast message");
            }

            message
        }

        /// Get the [`TypeId`] of the message payload
        fn payload_type(&self) -> TypeId {
            self.as_any().type_id()
        }
    }
}

/// Salish message trait, implemented by message containers for routing and unwrapping inner message types
pub trait SalishMessage {
    /// The [`Endpoint`] type that messages can be routed to
    type Endpoint: Endpoint;

    /// Implementations of [`SalishMessage`] must provide a method that returns a `&dyn Any`
    fn as_any(&self) -> &dyn Any;
}

/// Implement sealed trait [`SalishMessageInternal`] on anything implementing [`SalishMessage`]
impl<T> internal::SalishMessageInternal for T where T: SalishMessage {}

/// Message Payload
pub trait Payload: Any + std::fmt::Debug + 'static {}

/// Message Endpoint
pub trait Endpoint: std::fmt::Debug {
    type Addr;
    fn addr(&self) -> Self::Addr;
}
