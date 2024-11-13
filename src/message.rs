//! Message container for wrapping any [`Payload`]

use std::{
    any::{Any, TypeId},
    hash::{DefaultHasher, Hasher},
    marker::PhantomData,
    sync::Arc,
};

use crate::{
    policy::Policy,
    traits::{
        internal::SalishMessageInternal as _, BroadcastPayload, EndpointAddress, MessagePayload,
        Payload, SalishMessage, UnicastPayload,
    },
};

pub type DynMessageSource = Arc<dyn MessageSource>;

/// Message Source trait
pub trait MessageSource: Any + std::fmt::Debug + Send + Sync + 'static {
    fn as_any(&self) -> &dyn Any;
    fn hash(&self, state: &mut DefaultHasher);
}

/// Blanket implementation of MessageSource on supported trait bounds
impl<T> MessageSource for T
where
    T: Any + std::hash::Hash + std::fmt::Debug + Copy + Send + Sync,
{
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn hash(&self, state: &mut DefaultHasher)
    where
        Self: std::hash::Hash,
    {
        self.hash(state)
    }
}

/// Message container which wraps anything implementing [`Payload`].
pub struct Message {
    //source: Option<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
    source: Option<Arc<dyn MessageSource>>,
    dest: Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
    payload: MessagePayload,
    is_clone: bool,
}

impl Clone for Message {
    fn clone(&self) -> Self {
        match &self.payload {
            MessagePayload::Unicast(_) => {
                panic!("Cannot clone messages with Unicast payload");
            }
            MessagePayload::Broadcast(_) => Message {
                source: self.source.clone(),
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
            .field("source", &self.source)
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
            source: None,
            dest,
            payload,
            is_clone: false,
        }
    }

    /// Set the source address of this [`Message`]
    pub fn with_source(mut self, source: impl MessageSource + Copy) -> Self {
        self.source = Some(Arc::new(source));
        self
    }

    /// Set the destination address of this [`Message`]
    pub fn with_dest(
        mut self,
        dest: Destination<<<Self as SalishMessage>::Endpoint as EndpointAddress>::Addr>,
    ) -> Self {
        self.dest = dest;
        self
    }

    /// Check if the payload is of type T
    pub fn is_type<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == self.payload_type()
    }

    /// Get the hash of the source via trait object
    pub fn source_hash(&self) -> Option<u64> {
        if let Some(source) = &self.source {
            let mut hasher = DefaultHasher::new();
            source.hash(&mut hasher);
            Some(hasher.finish())
        } else {
            None
        }
    }

    /// Get the source of this message, downcast to the provided type
    pub fn source<T: Copy + 'static>(&self) -> Option<T> {
        if let Some(source) = &self.source {
            let source = (**source).as_any().downcast_ref::<T>().copied();
            if source.is_none() {
                panic!("Message source downcast failed");
            }
            source
        } else {
            None
        }
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

/// Convert a Result with a [`Payload`] type into a Message
impl<P, E> From<Result<P, E>> for Message
where
    P: Payload + 'static,
    E: std::error::Error + Payload + 'static,
{
    fn from(res: Result<P, E>) -> Self {
        match res {
            Ok(payload) => Message::unicast(payload),
            Err(err) => Message::unicast(err),
        }
    }
}
