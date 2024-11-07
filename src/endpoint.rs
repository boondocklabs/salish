use std::{
    marker::PhantomData,
    sync::{atomic::AtomicU64, Arc, LazyLock, Mutex},
};

use crate::{handler::MessageHandler, router::MessageRouter, traits::Payload};

static ENDPOINT_ID: LazyLock<Arc<AtomicU64>> = LazyLock::new(|| Arc::new(AtomicU64::new(0)));

pub type EndpointId = u64;

#[derive(Debug)]
pub struct Endpoint<'a, M, R>
where
    R: std::fmt::Debug + 'a,
    M: Payload,
{
    id: EndpointId,
    router: MessageRouter<'a, R>,
    endpoint: Arc<Mutex<EndpointInner<'a, M, R>>>,
}

impl<'a, M, R> Drop for Endpoint<'a, M, R>
where
    R: std::fmt::Debug + 'a,
    M: Payload,
{
    fn drop(&mut self) {
        println!("OUTER ENDPOINT DROPPED");

        self.router.remove_endpoint(self.id);
    }
}

/*
impl<'a, M, R> Clone for Endpoint<'a, M, R>
where
    R: std::fmt::Debug,
    M: Payload,
{
    fn clone(&self) -> Self {
        Endpoint {
            id: self.id,
            endpoint: self.endpoint.clone(),
        }
    }
}
*/

impl<'a, M, R> Endpoint<'a, M, R>
where
    R: std::fmt::Debug,
    M: Payload,
{
    pub fn new(router: MessageRouter<'a, R>) -> Self
    where
        R: 'a,
    {
        let endpoint = Self {
            id: ENDPOINT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            endpoint: Arc::new(Mutex::new(EndpointInner::new())),
            router,
        };

        endpoint
            .router
            .add_wrapped_handler(endpoint.id, endpoint.endpoint.clone());

        println!("Endpoint Created {endpoint:?}");

        endpoint
    }

    pub fn message<F>(self, f: F) -> Self
    where
        F: FnMut(&M) -> R + 'a,
    {
        self.endpoint.lock().unwrap().callback = Some(Box::new(f));
        self
    }
}

pub struct EndpointInner<'a, M, R>
where
    Self: MessageHandler,
    M: std::fmt::Debug,
{
    callback: Option<
        Box<dyn FnMut(&<Self as MessageHandler>::Message) -> <Self as MessageHandler>::Return + 'a>,
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
    M: std::fmt::Debug,
{
    fn drop(&mut self) {
        println!("INNER ENDPOINT DROPPED")
        // Deregister this handler from the router
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
    R: std::fmt::Debug,
{
    type Message = M;
    type Return = R;

    fn on_message<'b>(&'b mut self, message: &'b Self::Message) -> Self::Return {
        if let Some(callback) = &mut self.callback {
            (callback)(message)
        } else {
            panic!("No message handler defined in Endpoint")
        }
    }
}
