use std::{cell::RefCell, rc::Rc};

use tracing_test::traced_test;

use crate::{handler::MessageHandler, message::Message, router::MessageRouter};

use super::TestPayload;

#[derive(Default, Debug)]
struct TestHandler;

impl MessageHandler for TestHandler {
    type Message = TestPayload;
    type Return = bool;

    fn on_message(&mut self, message: &Self::Message) -> Self::Return {
        println!("HANDLER MESSAGE {message:#?}");
        true
    }
}

#[traced_test]
#[test]
fn handler() {
    let handler = Rc::new(RefCell::new(TestHandler::default()));

    let mut router = MessageRouter::new();
    router.add_wrapped_handler(0, handler);

    let _task = router.handle_message(&mut Message::new(TestPayload::String("hello")));
}
