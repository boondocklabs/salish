use crate::{
    message::{DynMessageSource, MessageSource},
    traits::Payload,
};

/// Message Handler Trait
pub trait MessageHandler: std::fmt::Debug + Send + Sync {
    /// Payload type this handler is receiving
    type Message: Payload;

    type Source: MessageSource + Copy;

    /// The return type of the message handler
    type Return;

    /// Called when a message is received
    fn on_message(&mut self, source: Option<Self::Source>, message: Self::Message) -> Self::Return;
}
