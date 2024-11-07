use std::{
    any::{type_name, Any, TypeId},
    hash::{DefaultHasher, Hasher as _},
    marker::PhantomData,
    rc::Rc,
    sync::Arc,
};

use crate::traits::{internal::SalishMessageInternal as _, Endpoint, Payload, SalishMessage};

#[derive(Clone)]
pub struct Message {
    dest: Destination<<<Self as SalishMessage>::Endpoint as Endpoint>::Addr>,
    data: Arc<Box<dyn Any>>,
    type_name: &'static str,
}

impl std::fmt::Debug for Message {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Message")
            .field("dest", &self.dest)
            .field("payload_type_id", &self.payload_type())
            .field("payload_type_name", &self.type_name)
            .finish()
    }
}

impl Message {
    /// Create a new message with destination set to [`Destination::Any`].
    /// This will route the message to any registered receiver for this message type
    pub fn new<T: Payload + 'static>(data: T) -> Self {
        Self::new_to(Destination::Any, data)
    }

    /// Create a new message with destination specified by `dest`
    pub fn new_to<T: Payload + 'static>(
        dest: Destination<<<Self as SalishMessage>::Endpoint as Endpoint>::Addr>,
        data: T,
    ) -> Self {
        Self {
            dest,
            data: Arc::new(Box::new(data)),
            type_name: type_name::<T>(),
        }
    }

    /// Check if the payload is of type T
    pub fn is_type<T: 'static>(&self) -> bool {
        TypeId::of::<T>() == self.payload_type()
    }
}

#[derive(Clone, Debug)]
pub enum Destination<Addr> {
    Any,
    Endpoint(Rc<Box<dyn Endpoint<Addr = Addr>>>),
}

impl<Addr: 'static> Destination<Addr> {
    pub fn any() -> Self {
        Self::Any
    }

    pub fn endpoint(endpoint: impl Endpoint<Addr = Addr> + 'static) -> Self {
        Self::Endpoint(Rc::new(Box::new(endpoint)))
    }
}

#[derive(Debug, Clone)]
struct HashEndpoint<'a, T>
where
    T: std::fmt::Debug + std::hash::Hash,
{
    h: &'a T,
    _phantom: PhantomData<T>,
}

impl<'a, T: std::hash::Hash> Endpoint for HashEndpoint<'a, T>
where
    T: std::fmt::Debug + std::hash::Hash,
{
    type Addr = u64;
    fn addr(&self) -> Self::Addr {
        let mut hasher = DefaultHasher::new();
        self.h.hash(&mut hasher);
        hasher.finish()
    }
}

impl Endpoint for u64 {
    type Addr = u64;

    fn addr(&self) -> Self::Addr {
        *self
    }
}

impl SalishMessage for Message {
    type Endpoint = u64;

    /// Return the payload to internal trait methods as &dyn Any
    fn as_any(&self) -> &dyn Any {
        &**self.data
    }
}
