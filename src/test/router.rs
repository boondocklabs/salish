use tracing_test::traced_test;

use crate::{message::Message, router::MessageRouter, test::TestPayload};

#[traced_test]
#[test]
fn create() {
    let mut router = MessageRouter::<&'static str, &'static str>::new();
    let msg = Message::unicast(TestPayload::Integer(1234)).with_source("test");
    let _ = router.handle_message(msg);
}
