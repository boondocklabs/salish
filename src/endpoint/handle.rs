//! Endpoint type erased handle

use std::ops::Deref;

use anylock::AnyLock;
use tracing::error;

use crate::{
    handler::MessageHandler as _,
    message::Message,
    traits::{internal::SalishMessageInternal as _, Payload},
};

use super::{Endpoint, EndpointId, EndpointInner};

/// Endpoint callback to the inner dispatch closure which downcasts to concrete message type
/// and forwards to [`Endpoint::on_message()`]
pub type EndpointCallbackOwned<'a, Ret> =
    Box<dyn Fn(crate::message::Message) -> Option<Ret> + Send + Sync + 'a>;

#[allow(unused)]
pub type EndpointCallbackRef<'a, Ret> =
    Box<dyn for<'b> Fn(&'b crate::message::Message) -> Option<Ret> + Send + Sync + 'a>;

/// Type erased endpoint handle. Contains a callback to the message handler
pub struct EndpointHandle<'a, Ret>
where
    Self: Send + Sync,
{
    pub endpoint_id: EndpointId,
    pub callback: EndpointCallbackOwned<'a, Ret>,
}

impl<'a, Ret> std::fmt::Debug for EndpointHandle<'a, Ret>
where
    Self: Send + Sync,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointHandle")
            .field("endpoint_id", &self.endpoint_id)
            .finish()
    }
}

impl<'a, Ret> EndpointHandle<'a, Ret>
where
    Self: Send + Sync,
{
    /// Create a new [`EndpointHandle`] for an [`Endpoint`]
    pub fn new<M: Payload, Lock, Ref>(endpoint: &Endpoint<'a, M, Ret, Lock, Ref>) -> Self
    where
        Ref: Deref<Target: AnyLock<EndpointInner<'a, M, Ret>>>
            + From<Lock>
            + Clone
            + Send
            + Sync
            + 'a,
        Lock: AnyLock<EndpointInner<'a, M, Ret>> + 'a,
    {
        // Get a clone of the [`EndpointInner`] handler, which can be held longer than the [`Endpoint`] itself
        let inner = endpoint.inner.clone();

        let dispatch = move |message: Message| {
            // Get the downcast inner concrete message of type [`MessageHandler::Message`]
            if let Some(payload) = message.into_inner::<M>() {
                Some(inner.write().on_message(payload))
            } else {
                error!("Failed to downcast message");
                None
            }
        };

        EndpointHandle {
            endpoint_id: endpoint.id,
            callback: Box::new(dispatch),
        }
    }
}
