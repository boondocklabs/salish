use std::iter::repeat_with;

use tracing_test::traced_test;

use crate::{
    message::{Destination, Message},
    router::MessageRouter,
    traits::EndpointAddress as _,
};

use super::TestPayload;

type TestSource = u64;

#[traced_test]
#[test]
fn endpoint() {
    let mut router = MessageRouter::<Result<u64, ()>, TestSource>::new();
    let _endpoint = router
        .create_endpoint::<TestPayload>()
        .message(|_src, msg| {
            println!("Received message {msg:?}");
            if let TestPayload::Integer(num) = msg {
                Ok(num + 1)
            } else {
                Err(())
            }
        });

    let results = router.handle_message(Message::unicast(TestPayload::Integer(1)));

    println!("After message handled {results:?}");
}

#[traced_test]
#[test]
fn endpoint_deregister() {
    let mut router = MessageRouter::<(), TestSource>::new();

    // Create a Vec of 100 endpoints
    let endpoints: Vec<_> = repeat_with(|| {
        router
            .create_endpoint::<TestPayload>()
            .message(|_src, _msg| {})
    })
    .take(100)
    .collect();

    let _ = router.handle_message(Message::unicast(TestPayload::Integer(1)));

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
    let mut router = MessageRouter::<u32, TestSource>::new();

    let endpoint = router
        .create_endpoint::<TestPayload>()
        .message(|_src, msg| {
            println!("ENDPOINT RX {msg:?}");
            8675309
        });

    let message = Message::unicast(TestPayload::Integer(1234))
        .with_dest(Destination::endpoint((&endpoint).addr()));

    let result = router.handle_message(message);
    assert!(result.is_some());
    assert!(result.unwrap()[0] == 8675309);

    let message = Message::broadcast(TestPayload::Integer(1234))
        .with_dest(Destination::endpoint(endpoint.addr()));

    let result = router.handle_message(message);
    assert!(result.is_some());
    assert!(result.unwrap()[0] == 8675309);

    // Sending TestPayload to an unknown address should not be handled by the endpoint
    let message =
        Message::unicast(TestPayload::Integer(1234)).with_dest(Destination::endpoint(99999));
    let result = router.handle_message(message);
    assert!(result.is_none());

    // Sending TestPayload to an unknown address should not be handled by the endpoint
    let message =
        Message::broadcast(TestPayload::Integer(1234)).with_dest(Destination::endpoint(99999));
    let result = router.handle_message(message);
    assert!(result.is_none());

    drop(endpoint);
}

#[traced_test]
#[test]
fn endpoint_boxed() {
    let mut router = MessageRouter::<u32, TestSource>::new();

    let endpoint = router.create_endpoint::<Box<u32>>().message(|_src, msg| {
        println!("ENDPOINT RX {msg:?}");
        8675309
    });

    // Sending the wrong message type in a box to an endpoint receiving a Box<u32> should yield no results
    let message = Message::broadcast(Box::new("this is the wrong type"))
        .with_dest(Destination::endpoint(endpoint.addr()));
    let result = router.handle_message(message);
    assert!(result.is_none());

    // Sending the correct message type in a box to an endpoint receiving a Box<u32> should yield results
    let message =
        Message::broadcast(Box::new(1u32)).with_dest(Destination::endpoint(endpoint.addr()));
    let result = router.handle_message(message);
    assert!(result.is_some());
    assert!(result.unwrap()[0] == 8675309);
}

#[traced_test]
#[test]
fn endpoint_box_dyn() {
    trait TestTrait: std::fmt::Debug + Send + Sync {
        fn get(&self) -> u32;
    }

    #[derive(Debug)]
    struct Test {
        val: u32,
    }

    impl TestTrait for Test {
        fn get(&self) -> u32 {
            self.val
        }
    }

    let mut router = MessageRouter::<u32, TestSource>::new();

    // Create an endpoint listening for Box<dyn TestTrait>
    let endpoint = router
        .create_endpoint::<Box<dyn TestTrait>>()
        .message(|_src, msg| {
            println!("ENDPOINT RX {msg:?}");
            8675309
        });

    // Cooerce into a boxed trait object with explicit type annotation
    let msg: Box<dyn TestTrait> = Box::new(Test { val: 1234 });

    // Sending the wrong message type in a box to an endpoint receiving a Box<u32> should yield no results
    let message = Message::unicast(msg).with_dest(Destination::endpoint(endpoint.addr()));
    let result = router.handle_message(message);
    assert!(result.is_some());
    assert!(result.unwrap()[0] == 8675309);
}
