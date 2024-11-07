use std::{any::TypeId, cell::RefCell, collections::HashMap, rc::Rc};
use tracing::{debug, error, instrument, warn};

use crate::{
    endpoint::{Endpoint, EndpointId},
    handler::{HandlerWrapper, MessageHandler},
    message::Message,
    traits::{internal::SalishMessageInternal as _, Payload},
};

pub(crate) type HandlerClosure<'a, Ret> =
    Box<dyn for<'b> FnMut(&'b mut Message) -> Option<Ret> + 'a>;

/// Message Router
pub struct MessageRouter<'a, R> {
    // Vec of tuples of (EndpointId, HandlerClosure) so we can remove handlers by EndpointId
    handlers: Rc<RefCell<HashMap<TypeId, Vec<(EndpointId, HandlerClosure<'a, R>)>>>>,
}

impl<'a, R> Clone for MessageRouter<'a, R> {
    fn clone(&self) -> Self {
        MessageRouter {
            handlers: self.handlers.clone(),
        }
    }
}

impl<'a, R> std::fmt::Debug for MessageRouter<'a, R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let handlers = self.handlers.borrow();

        // Get a Vec of endpoints and handler counts for debug
        let handlers_count: Vec<_> = handlers.iter().map(|(k, v)| (k, v.len())).collect();

        f.debug_struct("MessageRouter")
            .field("handlers", &handlers_count)
            .finish()
    }
}

impl<'a, R> MessageRouter<'a, R> {
    pub fn new() -> Self {
        Self {
            handlers: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Call a [`Vec`] of handlers with a reference to a [`Message`]
    fn call_handlers<'b>(
        message: &mut Message,
        //handlers: &mut Vec<Box<dyn for<'b> FnMut(&'b mut Message) -> Option<R> + 'a>>,
        handlers: &mut Vec<(EndpointId, HandlerClosure<'b, R>)>,
    ) -> Option<Vec<R>> {
        match handlers.len() {
            // If we have a single handler, get a ref to the only handler,
            // call the handler, and map the returned option into a single element vec,
            // or return None if the handler returned None
            1 => (handlers[0].1)(message).map(|ret| vec![ret]),

            // Otherwise, call each handler and collect the results
            _ => {
                let tasks: Vec<R> = handlers
                    .into_iter()
                    .filter_map(|handler| (handler.1)(message))
                    .collect();
                if tasks.is_empty() {
                    None
                } else {
                    Some(tasks)
                }
            }
        }
    }

    /// Handle a message, and route them to registered [`MessageHandler`] implementations
    #[instrument(name = "router")]
    pub fn handle_message<'b>(&'b mut self, message: &mut Message) -> Option<Vec<R>> {
        let mut handlers = self.handlers.borrow_mut();

        if let Some(handlers) = handlers.get_mut(&message.payload_type()) {
            Self::call_handlers(message, handlers)
        } else {
            warn!("No Handler");
            None
        }
    }

    /*
    #[instrument(name = "router")]
    pub fn add_handler<H>(&self, mut handler: H)
    where
        H: MessageHandler<Return = R> + 'a,
    {
        let type_id = TypeId::of::<H::Message>();
        debug!("Handler TypeId: {type_id:?}");

        // Register a closure for dispatching messages to the handler
        let dispatch = move |msg: &mut Message| {
            // Get the downcast inner concrete message of type [`MessageHandler::Message`]
            if let Some(payload) = msg.inner::<H::Message>() {
                Some(handler.on_message(payload))
            } else {
                error!("Failed to downcast message");
                None
            }
        };

        self.handlers
            .borrow_mut()
            .entry(type_id)
            .or_default()
            .push(Box::new(dispatch));

        debug!("Added Handler");
    }
    */

    #[instrument(name = "router")]
    pub fn remove_endpoint(&self, endpoint_id: EndpointId) {
        println!("Removing Endpoint ID {endpoint_id}");
        self.handlers
            .borrow_mut()
            .iter_mut()
            .for_each(|(_k, v)| v.retain(|h| h.0 != endpoint_id));
    }

    /// Register a handler to receive messages
    /// The provided handler is a [`HandlerWrapper`]. This allows different container types (Arc/Rc/RefCell/Mutex)
    /// to be passed when registering a handler, and provides the [`HandlerWrapper::inner()`] method to unwrap
    /// into the inner handler.
    #[instrument(name = "router")]
    pub fn add_wrapped_handler<H, W>(&self, endpoint_id: EndpointId, mut handler: W)
    where
        W: HandlerWrapper<H> + std::fmt::Debug + 'a,
        H: MessageHandler<Return = R> + 'a,
    {
        // Get the type of the handlers associated type Message
        let type_id = TypeId::of::<H::Message>();

        debug!("Handler TypeId: {type_id:?}");

        // Register a closure for dispatching messages to the handler
        let dispatch = move |msg: &mut Message| {
            if let Some(mut inner) = handler.inner_mut() {
                // Get the downcast inner concrete message of type [`MessageHandler::Message`]
                if let Some(payload) = msg.inner::<H::Message>() {
                    Some(inner.on_message(payload))
                } else {
                    error!("Failed to downcast message");
                    None
                }
            } else {
                None
            }
        };

        let mut handlers = self.handlers.borrow_mut();
        handlers
            .entry(type_id)
            .or_default()
            .push((endpoint_id, Box::new(dispatch)));

        debug!("Added Handler");
    }

    pub fn create_endpoint<M: Payload>(&self) -> Endpoint<'a, M, R>
    where
        R: std::fmt::Debug + 'static,
    {
        Endpoint::<'a, M, R>::new(self.clone())
    }
}
