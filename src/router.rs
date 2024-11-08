use anylock::{AnyLock, ParkingLotRwLock};
use std::{any::TypeId, collections::HashMap, ops::Deref, sync::Arc};
use tracing::{debug, instrument, trace, warn};

use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};

use crate::{
    endpoint::{handle::EndpointHandle, Endpoint, EndpointId, EndpointInner},
    message::Message,
    traits::{internal::SalishMessageInternal as _, EndpointAddress as _, Payload},
};

type HandlerList<'a, Ret> = Vec<EndpointHandle<'a, Ret>>;

const THREADS: usize = 4;

/// Message Router
pub struct MessageRouter<'a, R> {
    /// Registered endpoints by EndpointId
    endpoints: Arc<ParkingLotRwLock<HashMap<EndpointId, EndpointHandle<'a, R>>>>,

    /// Map of [`TypeId`] of the Message that an Endpoint is registered to receive.
    /// This is used to dispatch messages to all registered endpoints for a specific type.
    type_handlers: Arc<ParkingLotRwLock<HashMap<TypeId, HandlerList<'a, R>>>>,

    /// Rayon thread pool
    pool: Option<ThreadPool>,
}

impl<'a, R> Clone for MessageRouter<'a, R>
where
    R: Send,
{
    fn clone(&self) -> Self {
        MessageRouter {
            endpoints: self.endpoints.clone(),
            type_handlers: self.type_handlers.clone(),

            // Clones do not get a thread pool
            pool: None,
        }
    }
}

impl<'a, R> std::fmt::Debug for MessageRouter<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //let handlers = self.type_handlers.read();

        // Get a Vec of endpoints and handler counts for debug
        //let handlers_count: Vec<_> = handlers.iter().map(|(k, v)| (k, v.len())).collect();

        f.debug_struct("MessageRouter")
            .field("endpoints", &self.num_endpoints())
            //.field("handlers", &handlers_count)
            .finish()
    }
}

impl<'a, R> MessageRouter<'a, R> {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(ParkingLotRwLock::new(HashMap::new())),
            type_handlers: Arc::new(ParkingLotRwLock::new(HashMap::new())),
            pool: Some(Self::new_pool()),
        }
    }

    /// Create a new thread pool. Only the original MessageRouter obtains a pool.
    /// Clones of the router to keep references to endpoint lists for auto deregistration do not obtain a pool.
    fn new_pool() -> ThreadPool {
        ThreadPoolBuilder::new()
            .num_threads(THREADS)
            .build()
            .expect("Failed to create thread pool")
    }

    /// Get the number of endpoints registered with the router
    pub fn num_endpoints(&self) -> usize {
        // Sum the inner vec lengths for all keys
        self.endpoints.read().len()
    }

    /// Get the number of handlers registered with the router
    pub fn num_handlers(&self) -> usize {
        // Sum the inner vec lengths for all keys
        self.type_handlers
            .read()
            .iter()
            .map(|(_k, v)| v.len())
            .sum()
    }

    /// Call a [`Vec`] of handlers with a reference to a [`Message`]
    fn call_handlers<'b>(
        &self,
        message: &Message,
        //handlers: &mut Vec<Box<dyn for<'b> FnMut(&'b mut Message) -> Option<R> + 'a>>,
        handlers: &HandlerList<'b, R>,
    ) -> Option<Vec<R>>
    where
        R: Send,
    {
        match handlers.len() {
            0 => {
                warn!("No handlers");
                None
            }
            // If we have a single handler, get a ref to the only handler,
            // call the handler, and map the returned option into a single element vec,
            // or return None if the handler returned None
            1 => (handlers[0].callback)(message).map(|ret| vec![ret]),

            // Otherwise, call each handler and collect the results
            _ => self.pool.as_ref().unwrap().install(|| {
                let tasks: Vec<R> = handlers
                    .chunks((handlers.len() / THREADS).max(1))
                    .par_bridge()
                    .flat_map(|handler_batch| {
                        //println!("BATCH {}", handler_batch.len());
                        handler_batch
                            .iter()
                            .filter_map(|handler| (handler.callback)(message))
                            .collect::<Vec<R>>()
                    })
                    .collect();
                if tasks.is_empty() {
                    None
                } else {
                    Some(tasks)
                }
            }),
        }
    }

    /// Handle a message, and route them to registered [`MessageHandler`] implementations
    #[instrument(name = "router")]
    pub fn handle_message<'b>(&'b mut self, message: &mut Message) -> Option<Vec<R>>
    where
        R: Send,
    {
        match message.dest() {
            crate::message::Destination::Any => {
                let handlers = self.type_handlers.read();

                if let Some(handlers) = handlers.get(&message.payload_type()) {
                    self.call_handlers(message, handlers)
                } else {
                    warn!("No Handler");
                    None
                }
            }
            crate::message::Destination::Endpoint(endpoint) => {
                trace!("Sending to endpoint {}", endpoint.addr());

                if let Some(handle) = self.endpoints.read().get(&endpoint.addr()) {
                    (handle.callback)(message).map(|res| vec![res])
                } else {
                    None
                }
            }
        }
    }

    #[instrument(name = "router")]
    pub fn remove_endpoint(&self, endpoint_id: EndpointId) {
        debug!("Removing Endpoint ID {endpoint_id}");

        self.endpoints.write().remove(&endpoint_id);

        self.type_handlers
            .write()
            .iter_mut()
            .for_each(|(_k, v)| v.retain(|h| h.endpoint_id != endpoint_id));
    }

    /*
    /// Register a handler to receive messages
    /// Expects a type H implementing [`MessageHandler`] wrapped in somethings that derefs (Arc,Rc) to an [`AnyLock`]
    //#[instrument(name = "router")]
    pub fn add_handler<H, W>(&self, endpoint_id: EndpointId, handler: W)
    where
        W: Deref<Target: AnyLock<H>> + 'a + Send + Sync,
        H: MessageHandler<Return = R> + 'a,
    {
        // Get the type of the handlers associated type Message
        let type_id = TypeId::of::<H::Message>();

        //debug!("Handler TypeId: {type_id:?}");

        // Register a closure for dispatching messages to the handler
        let dispatch = move |msg: &Message| {
            let mut guard = handler.write();

            // Get the downcast inner concrete message of type [`MessageHandler::Message`]
            if let Some(payload) = msg.inner::<H::Message>() {
                Some(guard.on_message(payload))
            } else {
                error!("Failed to downcast message");
                None
            }
        };

        let handle = EndpointHandle {
            endpoint_id,
            callback: Box::new(dispatch),
        };

        let mut handlers = self.type_handlers.write();
        handlers.entry(type_id).or_default().push(handle);

        debug!("Added Handler");
    }
    */

    /// Add an [`EndpointHandle`] to the router
    fn add_endpoint_handle(&self, handle: EndpointHandle<'a, R>) {
        debug!("Adding {handle:?}");
        self.endpoints.write().insert(handle.endpoint_id, handle);
    }

    /// Add an [`Endpoint`] to the router. This is handled automatically in [`Endpoint::new()`]
    pub fn add_endpoint<M, Lock, Ref>(&self, endpoint: &Endpoint<'a, M, R, Lock, Ref>)
    where
        M: Payload,
        Ref: Deref<Target: AnyLock<EndpointInner<'a, M, R>>> + From<Lock> + Clone + Send + Sync,
        Lock: AnyLock<EndpointInner<'a, M, R>> + Send + Sync,
    {
        // Add the endpoint to the `endpoints` map
        self.add_endpoint_handle(endpoint.handle());

        // Add the endpoint based on message TypeId to `type_handlers`
        self.type_handlers
            .write()
            .entry(endpoint.message_type())
            .or_default()
            .push(endpoint.handle());

        debug!("{endpoint:?} Added");
    }

    /// Create a new [`Endpoint`] registered with this router
    #[instrument(name = "router")]
    pub fn create_endpoint<M: Payload>(&self) -> Endpoint<'a, M, R>
    where
        R: Send + 'static,
    {
        Endpoint::<'a, M, R>::new(self.clone())
    }
}
