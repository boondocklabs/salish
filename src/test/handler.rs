use crate::handler::MessageHandler;

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

/*
#[traced_test]
#[test]
fn handler_arc_mutex() {
    let handler: Arc<anylock::StdMutex<TestHandler>> =
        Arc::new(AnyLock::new(TestHandler::default()));

    let mut router = MessageRouter::new();
    router.add_handler(0, handler);

    let _task = router.handle_message(&mut Message::new(TestPayload::String("hello")));
}
*/

/*
#[traced_test]
#[test]
fn handler_rc_refcell() {
    let handler: Rc<core::cell::RefCell<TestHandler>> =
        Rc::new(AnyLock::new(TestHandler::default()));

    let mut router = MessageRouter::new();
    router.add_handler(0, handler);

    let _task = router.handle_message(&mut Message::new(TestPayload::String("hello")));
}
*/
