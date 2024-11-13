//! Message Endpoint

use std::{
    any::TypeId,
    marker::PhantomData,
    ops::Deref,
    sync::{atomic::AtomicU64, Arc, LazyLock},
};

use anylock::AnyLock;
use handle::EndpointHandle;
use tracing::debug;

use crate::{
    filter::Filter,
    handler::MessageHandler,
    message::MessageSource,
    router::MessageRouter,
    traits::{EndpointAddress, Payload},
};

pub(crate) mod handle;

static ENDPOINT_ID: LazyLock<Arc<AtomicU64>> = LazyLock::new(|| Arc::new(AtomicU64::new(0)));

pub type EndpointId = u64;

/// Message Endpoint
///
/// This is split into an outer Endpoint, and [`EndpointInner`] which implements [`MessageHandler`]
/// This allows the outer [`Endpoint`] to control deregistration on drop, as there are no clones of the outer [`Endpoint`].
/// The owner of the endpoint can drop the endpoint, which will deregister the endpoint from [`MessageRouter`].
pub struct Endpoint<
    'a,
    Message,
    Return,
    Source,
    // Default to ParkingLotMutex lock
    Lock = anylock::ParkingLotMutex<EndpointInner<'a, Message, Return, Source>>,
    // Default to Arc reference
    Ref = std::sync::Arc<Lock>,
> where
    Self: Send + Sync,
    Return: Send + 'a,
    Message: Payload,
    Source: MessageSource + Copy,

    // Ref can be anything that Derefs to AnyLock wrapping [`EndpointInner`]
    Ref: Deref<Target: AnyLock<EndpointInner<'a, Message, Return, Source>>>
        // From<Lock> allows using .into() on the lock to obtain a reference
        + From<Lock>
        + Send
        + Sync
        // Inner Ref must be cloneable
        + Clone
        + 'a,
    Lock: AnyLock<EndpointInner<'a, Message, Return, Source>> + Send + 'a,
{
    id: EndpointId,
    router: Option<MessageRouter<'a, Return, Source>>,
    inner: Ref,
    _phantom: (PhantomData<Message>, PhantomData<Source>, PhantomData<Lock>),
}

impl<'a, Message, Return, Source, Lock, Ref> EndpointAddress
    for Endpoint<'a, Message, Return, Source, Lock, Ref>
where
    Self: Send + Sync,
    Return: Send + 'a,
    Message: Payload,
    Source: MessageSource + Copy,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, Message, Return, Source>>>
        + From<Lock>
        + Send
        + Sync
        + Clone
        + 'a,
    Lock: AnyLock<EndpointInner<'a, Message, Return, Source>> + Send + 'a,
{
    type Addr = EndpointId;

    fn addr(&self) -> Self::Addr {
        self.id
    }
}

impl<'a, M, R, S, Lock, Ref> std::fmt::Debug for Endpoint<'a, M, R, S, Lock, Ref>
where
    Self: Send + Sync,
    R: Send + 'a,
    M: Payload,
    S: MessageSource + Copy,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R, S>>> + From<Lock> + Clone + Send + Sync + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R, S>> + Send + 'a,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint")
            .field("id", &self.id)
            .field("filters", &self.inner.read().filters())
            .finish()
    }
}

/// Automatically deregister ourselves from the [`MessageRouter`] on Drop
impl<'a, M, R, S, Lock, Ref> Drop for Endpoint<'a, M, R, S, Lock, Ref>
where
    Self: Send + Sync,
    R: Send,
    M: Payload,
    S: MessageSource + Copy,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R, S>>> + From<Lock> + Clone + Send + Sync + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R, S>> + Send + 'a,
{
    fn drop(&mut self) {
        if let Some(router) = &self.router {
            router.remove_endpoint(self.id);
        }
    }
}

/// [`Endpoint`] implementation
impl<'a, M, R, S, Lock, Ref> Endpoint<'a, M, R, S, Lock, Ref>
where
    Self: Send + Sync,
    R: Send + 'a,
    M: Payload + 'static,
    S: MessageSource + Copy,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R, S>>> + From<Lock> + Clone + Send + Sync + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R, S>> + Send + Sync + 'a,
{
    pub fn new(router: Option<MessageRouter<'a, R, S>>) -> Self
    where
        R: 'a,
    {
        let endpoint = Self {
            id: ENDPOINT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            inner: Lock::new(EndpointInner::new()).into(),
            router,
            _phantom: (PhantomData, PhantomData, PhantomData),
        };

        // Register this endpoint with the router
        if let Some(router) = endpoint.router() {
            router.add_endpoint(&endpoint);
        }

        let type_id = TypeId::of::<M>();

        debug!(
            "Created {endpoint:?} Addr: {:?} for {type_id:?}",
            endpoint.addr()
        );

        endpoint
    }

    /// Get a new [`EndpointHandle`] for this endpoint which erases the payload type
    pub fn handle(&self) -> EndpointHandle<'a, R, S> {
        EndpointHandle::new(self)
    }

    /// Get a reference to the [`MessageRouter`] which was cloned into this endpoint
    pub fn router(&self) -> Option<&MessageRouter<'a, R, S>> {
        self.router.as_ref()
    }

    // Get the [`TypeId`] of the messages this endpoint can receive
    pub fn message_type(&self) -> TypeId
    where
        M: 'static,
    {
        TypeId::of::<M>()
    }

    pub fn filter(self, filter: impl Filter + 'static) -> Self {
        self.inner.write().add_filter(filter);
        self
    }

    // Register a message callback with [`EndpointInner`]
    pub fn message<F>(self, f: F) -> Self
    where
        F: Fn(Option<S>, M) -> R + Send + Sync + 'a,
    {
        self.inner.write().callback = Some(Box::new(f));
        self
    }
}

/// Inner Endpoint. Clones of this can be held alive and not prevent [`Endpoint`] [`Drop`] impl from deregistering
/// the endpoint from the [`MessageRouter`].
pub struct EndpointInner<'a, M, R, S>
where
    Self: MessageHandler + Send + Sync,
{
    filters: Vec<Box<dyn Filter>>,
    callback: Option<
        Box<
            dyn Fn(Option<S>, <Self as MessageHandler>::Message) -> <Self as MessageHandler>::Return
                + Send
                + Sync
                + 'a,
        >,
    >,
    _phantom: PhantomData<M>,
}

impl<'a, M, R, S> std::fmt::Debug for EndpointInner<'a, M, R, S>
where
    Self: MessageHandler,
    M: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointInner").finish()
    }
}

impl<'a, M, R, S> Drop for EndpointInner<'a, M, R, S>
where
    Self: MessageHandler,
{
    fn drop(&mut self) {
        debug!("Inner endpoint handler dropped");
        // deregistered from outer [`Endpoint`] container
    }
}

impl<'a, M, R, S> EndpointInner<'a, M, R, S>
where
    Self: MessageHandler,
    M: Payload,
    R: 'a,
{
    pub fn new() -> Self {
        Self {
            filters: Vec::new(),
            callback: None,
            _phantom: PhantomData,
        }
    }

    pub fn add_filter(&mut self, filter: impl Filter + 'static) {
        self.filters.push(Box::new(filter))
    }

    /// Get the filters assigned to this inner endpoint
    pub fn filters(&self) -> &Vec<Box<dyn Filter>> {
        &self.filters
    }

    pub fn filter(&self, message: &crate::Message) -> bool {
        for filter in &self.filters {
            let res = filter.filter(message);
            if res == true {
                println!("ENDPOINT FILTER MATCH {filter:?}");
                return true;
            }
        }
        false
    }
}

impl<'a, M, R, S> MessageHandler for EndpointInner<'a, M, R, S>
where
    M: Payload,
    S: MessageSource + Copy,
{
    type Message = M;
    type Return = R;
    type Source = S;

    fn on_message(&mut self, source: Option<Self::Source>, message: Self::Message) -> Self::Return {
        if let Some(callback) = &mut self.callback {
            (callback)(source, message)
        } else {
            panic!("No message handler defined in Endpoint. Ensure you've registered a closure with Endpoint::message()")
        }
    }
}
