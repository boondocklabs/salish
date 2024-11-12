//! Message container for wrapping any [`Payload`]

use std::{
    any::TypeId,
    hash::{DefaultHasher, Hasher as _},
    marker::PhantomData,
};

use crate::{
    policy::Policy,
    traits::{
        internal::SalishMessageInternal as _, BroadcastPayload, EndpointAddress, MessagePayload,
        SalishMessage, UnicastPayload,
    },
};

/// Message container which wraps anything implementing [`Payload`].
pub struct Message {
    dest: Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
    payload: MessagePayload,
    is_clone: bool,
}

impl Clone for Message
where
    Self: Send + Sync,
{
    fn clone(&self) -> Self {
        match &self.payload {
            MessagePayload::Unicast(_) => {
                panic!("Cannot clone messages with Unicast payload");
            }
            MessagePayload::Broadcast(_) => Message {
                dest: self.dest,
                payload: self.payload.clone(),
                is_clone: true,
            },
        }
    }
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = &mut f.debug_struct("Message");
        debug = debug
            .field("dest", &self.dest)
            .field("payload_id", &self.payload_type())
            .field("payload", &self.payload);

        if self.is_clone {
            debug = debug.field("cloned", &self.is_clone)
        }

        debug.finish()
    }
}

impl Message {
    /// Create a new message with destination set to [`Destination::Any`].
    /// This will route the message to any registered receiver for this message type
    pub fn unicast<P: UnicastPayload + 'static>(payload: P) -> Self {
        Self::new_to(Destination::Any(Policy::default()), payload.into_payload())
    }

    /// Create a new message with destination set to [`Destination::Broadcast`]
    pub fn broadcast<P: BroadcastPayload + 'static>(payload: P) -> Self {
        Self::new_to(
            Destination::Broadcast(Policy::default()),
            payload.into_payload(),
        )
    }

    /// Create a new message with destination specified by `dest`
    pub fn new_to(
        dest: Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
        payload: MessagePayload,
    ) -> Self {
        Self {
            dest,
            payload,
            is_clone: false,
        }
    }

    /// Check if the payload is of type T
    pub fn is_type<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == self.payload_type()
    }

    /// Get the destination of this message
    pub fn dest(
        &self,
    ) -> Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr> {
        self.dest
    }
}

impl SalishMessage for Message {
    type Endpoint = u64;

    fn payload<'b>(&'b self) -> &'b MessagePayload {
        &self.payload
    }

    fn to_payload(self) -> MessagePayload {
        self.payload
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Destination<Addr> {
    /// Message destined to any endpoint listening to a message type.
    /// It will be delivered to a single endpoint only. Use broadcast
    /// to send clones to all endpoints listening to a message type.
    Any(Policy),

    /// Broadcast clones of the message to all endpoints registered for
    /// the payload [`TypeId`] of the message
    Broadcast(Policy),

    /// Publish a message to subscribers of a [`Topic`]
    //Publish

    /// Message destined to a specific endpoint
    //Endpoint(Arc<dyn EndpointAddress<Addr = Addr>>),
    Endpoint(Addr),
}

impl<Addr: 'static> Destination<Addr> {
    pub fn any() -> Self {
        Self::Any(Policy::default())
    }

    pub fn endpoint(addr: Addr) -> Self {
        Self::Endpoint(addr)
    }
}

#[derive(Debug, Clone)]
struct HashEndpoint<'a, T>
where
    T: std::fmt::Debug + std::hash::Hash + Send + Sync,
{
    h: &'a T,
    _phantom: PhantomData<T>,
}

impl<'a, T: std::hash::Hash> EndpointAddress for HashEndpoint<'a, T>
where
    T: std::fmt::Debug + std::hash::Hash + Send + Sync,
{
    type Addr = u64;
    fn addr(&self) -> Self::Addr {
        let mut hasher = DefaultHasher::new();
        self.h.hash(&mut hasher);
        hasher.finish()
    }
}

impl EndpointAddress for u64 {
    type Addr = u64;

    fn addr(&self) -> Self::Addr {
        *self
    }
}
