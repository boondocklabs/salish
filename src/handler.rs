use std::{
    cell::RefCell,
    ops::{Deref, DerefMut},
    rc::Rc,
    sync::{Arc, Mutex},
};

use crate::traits::Payload;

// Define a trait to wrap both Rc<RefCell<T>> and Arc<Mutex<T>>
pub trait HandlerWrapper<T> {
    fn inner(&self) -> Option<Box<dyn Deref<Target = T> + '_>>;
    fn inner_mut(&mut self) -> Option<Box<dyn DerefMut<Target = T> + '_>>;
}

// Implement for Rc<RefCell<T>>
impl<T> HandlerWrapper<T> for Rc<RefCell<T>> {
    fn inner(&self) -> Option<Box<dyn Deref<Target = T> + '_>> {
        Some(Box::new(self.borrow()))
    }

    fn inner_mut(&mut self) -> Option<Box<dyn DerefMut<Target = T> + '_>> {
        Some(Box::new(self.borrow_mut()))
    }
}

// Implement for Arc<Mutex<T>>
impl<T> HandlerWrapper<T> for Arc<Mutex<T>> {
    fn inner(&self) -> Option<Box<dyn Deref<Target = T> + '_>> {
        let guard = self.lock().unwrap();
        Some(Box::new(guard))
    }

    fn inner_mut(&mut self) -> Option<Box<dyn DerefMut<Target = T> + '_>> {
        let guard = self.lock().unwrap();
        Some(Box::new(guard))
    }
}

impl<T: MessageHandler> HandlerWrapper<T> for T {
    fn inner(&self) -> Option<Box<dyn Deref<Target = T> + '_>> {
        Some(Box::new(self))
    }

    fn inner_mut(&mut self) -> Option<Box<dyn DerefMut<Target = T> + '_>> {
        Some(Box::new(self))
    }
}

/// Message Handler Trait
pub trait MessageHandler: std::fmt::Debug {
    /// Payload type this handler is receiving
    type Message: Payload;

    /// The return type of the message handler
    type Return;

    fn on_message<'b>(&'b mut self, message: &'b Self::Message) -> Self::Return;
}
