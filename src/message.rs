//! Message container for wrapping any [`Payload`]

use std::{
    any::{type_name, TypeId},
    hash::{DefaultHasher, Hasher as _},
    marker::PhantomData,
};

use crate::{
    policy::Policy,
    traits::{internal::SalishMessageInternal as _, EndpointAddress, Payload, SalishMessage},
};

/// Message container which wraps anything implementing [`Payload`].
pub struct Message
where
    Self: Send + Sync,
{
    dest: Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
    payload: Box<dyn Payload>,
    type_name: &'static str,
    is_clone: bool,
}

impl Clone for Message
where
    Self: Send + Sync,
{
    fn clone(&self) -> Self {
        Self {
            dest: self.dest,
            payload: self.payload.clone_payload(),
            type_name: self.type_name,
            is_clone: true,
        }
    }
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug = &mut f.debug_struct("Message");
        debug = debug
            .field("dest", &self.dest)
            .field("payload_type_id", &self.payload_type())
            .field("payload_type_name", &self.type_name);

        if self.is_clone {
            debug = debug.field("cloned", &self.is_clone)
        }

        debug.finish()
    }
}

impl Message {
    /// Create a new message with destination set to [`Destination::Any`].
    /// This will route the message to any registered receiver for this message type
    pub fn new<P: Payload + 'static>(payload: P) -> Self {
        Self::new_to(Destination::Any(Policy::default()), payload)
    }

    /// Create a new message with destination set to [`Destination::Broadcast`]
    pub fn broadcast<P: Payload + 'static>(payload: P) -> Self {
        Self::new_to(Destination::Broadcast(Policy::default()), payload)
    }

    /// Create a new message with destination specified by `dest`
    pub fn new_to<P: Payload + 'static>(
        dest: Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
        payload: P,
    ) -> Self {
        Self {
            dest,
            //payload: Arc::new(Box::new(payload)),
            payload: Box::new(payload),
            type_name: type_name::<P>(),
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

    fn payload(&self) -> &dyn Payload {
        &*self.payload
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
