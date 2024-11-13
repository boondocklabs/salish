//! Endpoint type erased handle

use std::{any::TypeId, ops::Deref};

use anylock::AnyLock;
use tracing::{error, warn};

use crate::{
    handler::MessageHandler as _,
    message::{Message, MessageSource},
    traits::{internal::SalishMessageInternal as _, Payload},
};

use super::{Endpoint, EndpointId, EndpointInner};

/// Endpoint callback to the inner dispatch closure which downcasts to concrete message type
/// and forwards to [`Endpoint::on_message()`]
pub type EndpointCallbackOwned<'a, Ret, Source> =
    Box<dyn Fn(Option<Source>, crate::message::Message) -> Option<Ret> + Send + Sync + 'a>;

#[allow(unused)]
pub type EndpointCallbackRef<'a, Ret> =
    Box<dyn for<'b> Fn(&'b crate::message::Message) -> Option<Ret> + Send + Sync + 'a>;

pub type FilterCallback<'a> = Box<dyn for<'b> Fn(&'b crate::Message) -> bool + Send + Sync + 'a>;

/// Type erased endpoint handle. Contains a callback to the message handler
pub struct EndpointHandle<'a, Ret, Source>
where
    Source: MessageSource,
{
    pub endpoint_id: EndpointId,
    pub callback: EndpointCallbackOwned<'a, Ret, Source>,
    pub filter: FilterCallback<'a>,
}

impl<'a, Ret, Source> std::fmt::Debug for EndpointHandle<'a, Ret, Source>
where
    Source: MessageSource,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointHandle")
            .field("endpoint_id", &self.endpoint_id)
            .finish()
    }
}

impl<'a, Ret, Source> EndpointHandle<'a, Ret, Source>
where
    Source: MessageSource + Copy,
{
    /// Create a new [`EndpointHandle`] for an [`Endpoint`]
    pub fn new<M, Lock, Ref>(endpoint: &Endpoint<'a, M, Ret, Source, Lock, Ref>) -> Self
    where
        M: Payload + 'static,
        Ref: Deref<Target: AnyLock<EndpointInner<'a, M, Ret, Source>>>
            + From<Lock>
            + Clone
            + Send
            + Sync
            + 'a,
        Lock: AnyLock<EndpointInner<'a, M, Ret, Source>> + Send + Sync + 'a,
        Ret: Send,
    {
        // Get a clone of the [`EndpointInner`] handler, which can be held longer than the [`Endpoint`] itself
        let inner = endpoint.inner.clone();

        let dispatch = move |source: Option<Source>, message: Message| {
            if TypeId::of::<M>() != message.payload_type() {
                warn!(
                    "EndpointHandle message payload type {:?} != endpoint type {:?}",
                    message.payload_type(),
                    TypeId::of::<M>()
                );
                return None;
            }

            let mut guard = inner.write();
            // Get the downcast inner concrete message of type [`MessageHandler::Message`]
            if let Some(payload) = message.into_inner::<M>() {
                Some(guard.on_message(source, payload))
            } else {
                error!("Endpoint closure failed to downcast message");
                None
            }
        };

        let inner = endpoint.inner.clone();
        let filter = move |message: &crate::Message| {
            let guard = inner.write();
            guard.filter(message)
        };

        EndpointHandle {
            endpoint_id: endpoint.id,
            callback: Box::new(dispatch),
            filter: Box::new(filter),
        }
    }
}
