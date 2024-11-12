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
    handler::MessageHandler,
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
    M,
    R,
    // Default to ParkingLotMutex lock
    Lock = anylock::ParkingLotMutex<EndpointInner<'a, M, R>>,
    // Default to Arc reference
    Ref = std::sync::Arc<Lock>,
> where
    R: 'a,
    M: Payload,

    // Ref can be anything that Derefs to AnyLock wrapping [`EndpointInner`]
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R>>>
        // From<Lock> allows using .into() on the lock to obtain a reference
        + From<Lock>
        // Inner Ref must be cloneable
        + Clone
        + Send
        + Sync
        + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R>> + 'a,
{
    id: EndpointId,
    router: Option<MessageRouter<'a, R>>,
    inner: Ref,
    _phantom: (PhantomData<M>, PhantomData<Lock>),
}

impl<'a, M, R, Lock, Ref> EndpointAddress for Endpoint<'a, M, R, Lock, Ref>
where
    R: 'a,
    M: Payload,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R>>> + From<Lock> + Clone + Send + Sync,
    Lock: AnyLock<EndpointInner<'a, M, R>> + Send + Sync,
{
    type Addr = EndpointId;

    fn addr(&self) -> Self::Addr {
        self.id
    }
}

impl<'a, M, R, Lock, Ref> std::fmt::Debug for Endpoint<'a, M, R, Lock, Ref>
where
    R: 'a,
    M: Payload,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R>>> + From<Lock> + Clone + Send + Sync + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R>>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Endpoint").field("id", &self.id).finish()
    }
}

/// Automatically deregister ourselves from the [`MessageRouter`] on Drop
impl<'a, M, R, Lock, Ref> Drop for Endpoint<'a, M, R, Lock, Ref>
where
    R: 'a,
    M: Payload,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R>>> + From<Lock> + Clone + Send + Sync + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R>>,
{
    fn drop(&mut self) {
        if let Some(router) = &self.router {
            router.remove_endpoint(self.id);
        }
    }
}

/// [`Endpoint`] implementation
impl<'a, M, R, Lock, Ref> Endpoint<'a, M, R, Lock, Ref>
where
    M: Payload + 'static,
    Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R>>> + From<Lock> + Clone + Send + Sync + 'a,
    Lock: AnyLock<EndpointInner<'a, M, R>> + Send + Sync + 'a,
{
    pub fn new(router: Option<MessageRouter<'a, R>>) -> Self
    where
        R: 'a,
    {
        let endpoint = Self {
            id: ENDPOINT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            inner: Lock::new(EndpointInner::new()).into(),
            router,
            _phantom: (PhantomData, PhantomData),
        };

        // Register this endpoint with the router
        if let Some(router) = endpoint.router() {
            router.add_endpoint(&endpoint);
        }

        debug!("Created {endpoint:?}");

        endpoint
    }

    /// Get a new [`EndpointHandle`] for this endpoint which erases the payload type
    pub fn handle(&self) -> EndpointHandle<'a, R> {
        EndpointHandle::new(self)
    }

    /// Get a reference to the [`MessageRouter`] which was cloned into this endpoint
    pub fn router(&self) -> Option<&MessageRouter<'a, R>> {
        self.router.as_ref()
    }

    // Get the [`TypeId`] of the messages this endpoint can receive
    pub fn message_type(&self) -> TypeId
    where
        M: 'static,
    {
        TypeId::of::<M>()
    }

    // Register a message callback with [`EndpointInner`]
    pub fn message<F>(self, f: F) -> Self
    where
        F: FnMut(M) -> R + Send + Sync + 'a,
    {
        self.inner.write().callback = Some(Box::new(f));
        self
    }
}

/// Inner Endpoint. Clones of this can be held alive and not prevent [`Endpoint`] [`Drop`] impl from deregistering
/// the endpoint from the [`MessageRouter`].
pub struct EndpointInner<'a, M, R>
where
    Self: MessageHandler,
{
    callback: Option<
        Box<
            dyn FnMut(<Self as MessageHandler>::Message) -> <Self as MessageHandler>::Return
                + Send
                + Sync
                + 'a,
        >,
    >,
    _phantom: PhantomData<M>,
}

impl<'a, M, R> std::fmt::Debug for EndpointInner<'a, M, R>
where
    Self: MessageHandler,
    M: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EndpointHandler").finish()
    }
}

impl<'a, M, R> Drop for EndpointInner<'a, M, R>
where
    Self: MessageHandler,
{
    fn drop(&mut self) {
        debug!("Inner endpoint handler dropped");
        // deregistered from outer [`Endpoint`] container
    }
}

impl<'a, M, R> EndpointInner<'a, M, R>
where
    Self: MessageHandler,
    M: Payload,
    R: 'a,
{
    pub fn new() -> Self {
        Self {
            //router,
            callback: None,
            _phantom: PhantomData,
        }
    }
}

impl<'a, M, R> MessageHandler for EndpointInner<'a, M, R>
where
    M: Payload,
{
    type Message = M;
    type Return = R;

    fn on_message<'b>(&'b mut self, message: Self::Message) -> Self::Return {
        if let Some(callback) = &mut self.callback {
            (callback)(message)
        } else {
            panic!("No message handler defined in Endpoint. Ensure you've registered a closure with Endpoint::message()")
        }
    }
}
