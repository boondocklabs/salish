//! Message handling and routing to [`Endpoint`] instances

use anylock::{AnyLock, ParkingLotMutex, ParkingLotRwLock};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    ops::Deref,
    sync::{Arc, LazyLock},
};
use tracing::{debug, instrument, trace, warn};

use rayon::{prelude::*, ThreadPool, ThreadPoolBuilder};

use crate::{
    endpoint::{handle::EndpointHandle, Endpoint, EndpointId, EndpointInner},
    message::{Destination, Message},
    policy::Policy,
    traits::{internal::SalishMessageInternal as _, EndpointAddress as _, Payload},
};

use rand::prelude::*;

type HandlerList<'a, Ret> = Vec<EndpointHandle<'a, Ret>>;

const THREADS: usize = 4;

static STATIC_ENDPOINTS: LazyLock<Arc<ParkingLotMutex<Vec<Box<dyn Any + Send + Sync>>>>> =
    LazyLock::new(|| Arc::new(ParkingLotMutex::new(Vec::new())));

#[derive(Debug)]
struct TypeHandler<'a, R> {
    handlers: HandlerList<'a, R>,

    // Next index for round robin policy
    next_index: usize,
}

impl<'a, R> Default for TypeHandler<'a, R> {
    fn default() -> Self {
        Self {
            handlers: HandlerList::default(),
            next_index: 0,
        }
    }
}

/// Message Router
pub struct MessageRouter<'a, R> {
    /// Registered endpoints by EndpointId
    endpoints: Arc<ParkingLotRwLock<HashMap<EndpointId, EndpointHandle<'a, R>>>>,

    /// Map of [`TypeId`] of the Message that an Endpoint is registered to receive.
    /// This is used to dispatch messages to all registered endpoints for a specific type.
    type_handlers: Arc<ParkingLotRwLock<HashMap<TypeId, TypeHandler<'a, R>>>>,

    /// Static endpoints being held. These cannot be deregistered, and live as long as the router
    static_endpoints: Option<Vec<Box<dyn Any + Send + Sync>>>,

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

            // Static endpoints do not get cloned
            static_endpoints: None,
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
            .field("handlers", &self.num_handlers())
            .finish()
    }
}

impl<'a, R> MessageRouter<'a, R> {
    pub fn new() -> Self {
        Self {
            endpoints: Arc::new(ParkingLotRwLock::new(HashMap::new())),
            type_handlers: Arc::new(ParkingLotRwLock::new(HashMap::new())),
            static_endpoints: Some(Vec::new()),
            pool: Some(Self::new_pool()),
        }
    }

    /// Create a new thread pool. Only the original MessageRouter obtains a pool.
    /// Clones of the router to keep references to endpoint lists for auto deregistration do not obtain a pool.
    fn new_pool() -> ThreadPool {
        ThreadPoolBuilder::new()
            .num_threads(THREADS)
            .start_handler(|index| {
                debug!("Thread {index} started");
            })
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
            .map(|(_k, v)| v.handlers.len())
            .sum()
    }

    /// Call a [`Vec`] of handlers with a reference to a [`Message`]
    fn call_handlers<'b>(
        &self,
        message: Message,
        handlers: &HandlerList<'b, R>,
        _policy: Policy,
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
                let mut tasks: Vec<R> = vec![];

                /*
                tasks.extend(
                    handlers
                        .iter()
                        .filter_map(|handler| (handler.callback)(message.clone())),
                );
                */

                tasks.par_extend(
                    handlers
                        .par_iter()
                        .with_min_len(
                            handlers.len() / self.pool.as_ref().unwrap().current_num_threads(),
                        )
                        .filter_map(move |handler| (handler.callback)(message.clone())),
                );

                /*
                tasks.par_extend(
                    handlers
                        .par_iter()
                        .with_min_len(
                            handlers.len() / self.pool.as_ref().unwrap().current_num_threads(),
                        )
                        .fold(
                            || Vec::new(),
                            |mut v: Vec<R>, handler| {
                                if let Some(result) = (handler.callback)(message.clone()) {
                                    v.push(result);
                                }
                                v
                            },
                        )
                        .flatten(), //.filter_map(|handler| (handler.callback)(message.clone())),
                );
                */

                //let tasks: Vec<_> = handlers
                /*
                tasks.par_extend(
                    handlers
                        .par_chunks(
                            handlers.len() / self.pool.as_ref().unwrap().current_num_threads(),
                        )
                        .into_par_iter()
                        .flat_map(|handler_batch| {
                            //println!("BATCH {}", handler_batch.len());
                            handler_batch
                                .into_iter()
                                .filter_map(|handler| (handler.callback)(message.clone()))
                                .collect::<Vec<R>>()
                        }),
                );
                */

                if tasks.is_empty() {
                    None
                } else {
                    Some(tasks)
                }
            }),
        }
    }

    fn dispatch_any(&self, message: Message, policy: Policy) -> Option<Vec<R>> {
        if let Some(type_handler) = self.type_handlers.write().get_mut(&message.payload_type()) {
            match policy {
                Policy::RoundRobin => {
                    let handle = &type_handler.handlers
                        [type_handler.next_index % type_handler.handlers.len()];

                    type_handler.next_index = type_handler.next_index.wrapping_add(1);

                    (handle.callback)(message).map(|res| vec![res])
                }
                Policy::Random => {
                    let index = ThreadRng::default().gen_range(0..type_handler.handlers.len());
                    let handle = &type_handler.handlers[index];
                    (handle.callback)(message).map(|res| vec![res])
                }
            }
        } else {
            warn!(
                "No handlers for type {:?} dest {:?}",
                message.payload_type(),
                message.dest()
            );
            None
        }
    }

    fn dispatch_broadcast(&self, message: Message, policy: Policy) -> Option<Vec<R>>
    where
        R: Send,
    {
        // Broadcast clones to all endpoints registered for the [`TypeId`] of the incoming message
        let type_handlers = self.type_handlers.read();

        if let Some(type_handler) = type_handlers.get(&message.payload_type()) {
            if type_handler.handlers.len() == 1 {
                drop(type_handlers);
                return self.dispatch_any(message, policy);
            }

            self.call_handlers(message, &type_handler.handlers, policy)
        } else {
            warn!("No Handler");
            None
        }
    }

    /// Handle a message, and route them to registered [`MessageHandler`] implementations
    #[instrument(name = "router")]
    pub fn handle_message<'b>(&'b mut self, message: Message) -> Option<Vec<R>>
    where
        R: Send,
    {
        trace!("{message:?}");
        match message.dest() {
            // Deliver to a single destination endpoint registered for the message type
            Destination::Any(policy) => self.dispatch_any(message, policy),

            // Deliver to all endpoints registered for the message type
            Destination::Broadcast(policy) => self.dispatch_broadcast(message, policy),

            // Deliver to a specific [`EndpointId`]
            Destination::Endpoint(endpoint) => {
                trace!("Sending to endpoint {}", endpoint.addr());

                if let Some(handle) = self.endpoints.read().get(&endpoint.addr()) {
                    (handle.callback)(message).map(|res| vec![res])
                } else {
                    None
                }
            }
        }
    }

    /// Remove a registered [`Endpoint`] from the [`MessageRouter`] specified by [`EndpointId`]
    #[instrument(name = "router")]
    pub fn remove_endpoint(&self, endpoint_id: EndpointId) {
        debug!("Removing Endpoint ID {endpoint_id}");

        self.endpoints.write().remove(&endpoint_id);

        // Remove the EndpointId from the TypeId handler map
        // If this was the last entry being removed from a TypeId handler, we need to remove the TypeId from the map
        // We can do this with nested retain, one for the outer map, and one for the inner vec of EndpointHandle
        self.type_handlers.write().retain(|_k, v| {
            v.handlers.retain(|h| h.endpoint_id != endpoint_id);
            !v.handlers.is_empty() // Keep only if there are remaining handlers
        });
    }

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
            .handlers
            .push(endpoint.handle());

        debug!("{endpoint:?} Added");
    }

    /// Create a new [`Endpoint`] registered with this router
    #[instrument(name = "router")]
    pub fn create_endpoint<M: Payload>(&self) -> Endpoint<'a, M, R>
    where
        R: Send + 'a,
    {
        Endpoint::<'a, M, R>::new(Some(self.clone()))
    }

    /// Create a static endpoint that does not need to be held by the caller.
    /// It will be held in a vec of the router, and cannot be deregistered.
    pub fn static_endpoint<M: Payload, F>(&mut self, f: F)
    where
        R: Send + 'static,
        F: FnMut(M) -> R + Send + Sync + 'static,
    {
        //let handle = self.create_endpoint::<M>().message(f);
        let endpoint = Endpoint::<'static, M, R>::new(None).message(f);
        self.add_endpoint_handle(endpoint.handle());

        // Add the endpoint based on message TypeId to `type_handlers`
        self.type_handlers
            .write()
            .entry(endpoint.message_type())
            .or_default()
            .handlers
            .push(endpoint.handle());

        if let Some(static_endpoints) = &mut self.static_endpoints {
            static_endpoints.push(Box::new(endpoint));
            debug!("Static endpoint added");
        }

        debug!("{self:#?}");
    }
}
