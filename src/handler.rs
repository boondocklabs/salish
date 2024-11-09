use crate::traits::Payload;

/// Message Handler Trait
pub trait MessageHandler: std::fmt::Debug {
    /// Payload type this handler is receiving
    type Message: Payload;

    /// The return type of the message handler
    type Return;

    fn on_message<'b>(&'b mut self, message: Self::Message) -> Self::Return;
}
