use tracing_test::traced_test;

use crate::{message::Message, router::MessageRouter, test::TestPayload};

#[traced_test]
#[test]
fn create() {
    let mut router = MessageRouter::<&'static str>::new();
    let mut msg = Message::new(TestPayload::Integer(1234));
    let _ = router.handle_message(&mut msg);
}
