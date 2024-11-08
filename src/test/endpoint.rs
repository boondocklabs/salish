use std::{
    iter::repeat_with,
    sync::{Arc, Mutex},
};

use tracing_test::traced_test;

use crate::{
    message::{Destination, Message},
    router::MessageRouter,
    traits::EndpointAddress as _,
};

use super::TestPayload;

#[traced_test]
#[test]
fn endpoint() {
    let mut router = MessageRouter::<Result<u64, ()>>::new();
    let _endpoint = router.create_endpoint::<TestPayload>().message(|msg| {
        println!("Received message {msg:?}");
        if let TestPayload::Integer(num) = msg {
            Ok(num + 1)
        } else {
            Err(())
        }
    });

    let results = router.handle_message(&mut Message::new(TestPayload::Integer(1)));

    println!("After message handled {results:?}");
}

#[traced_test]
#[test]
fn endpoint_deregister() {
    let mut router = MessageRouter::<()>::new();

    // Create a Vec of 100 endpoints
    let endpoints: Vec<_> =
        repeat_with(|| router.create_endpoint::<TestPayload>().message(|_msg| {}))
            .take(100)
            .collect();

    let _ = router.handle_message(&mut Message::new(TestPayload::Integer(1)));

    // We should have 100 handlers
    assert_eq!(router.num_handlers(), 100);

    // And 100 endpoints
    assert_eq!(router.num_endpoints(), 100);

    // If we drop the endpoint, it should deregister itself from the router
    drop(endpoints);

    // The router should now have zero handlers after the endpoint deregistered itself on drop
    assert_eq!(router.num_handlers(), 0);
    assert_eq!(router.num_endpoints(), 0);
}

#[traced_test]
#[test]
fn endpoint_address() {
    let rx = Arc::new(Mutex::new(None));
    let mut router = MessageRouter::<()>::new();

    let endpoint = router.create_endpoint::<TestPayload>().message(|msg| {
        println!("ENDPOINT RX {msg:?}");
        *rx.clone().lock().unwrap() = Some(true);
    });
    endpoint.addr();

    let mut message = Message::new_to(
        Destination::endpoint(endpoint.addr()),
        TestPayload::Integer(1234),
    );

    router.handle_message(&mut message);

    let received = rx.lock().unwrap();
    assert!(received.is_some());

    drop(endpoint);
}
