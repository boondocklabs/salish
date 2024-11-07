//! Example of application message routing

use std::{
    sync::{atomic::AtomicU64, Arc},
    //thread::sleep,
    //time::Duration,
};

use salish::{endpoint::Endpoint, message::Message, router::MessageRouter, traits::Payload};

/// Say we have an App struct representing some application state
struct App<'a> {
    // Application message router, yielding a Task from each message handler
    pub router: MessageRouter<'static, Task>,

    temp_endpoints: Vec<Endpoint<'a, TempMessage, Task>>,

    count: Arc<AtomicU64>,
}

impl<'a> App<'a> {
    fn new() -> Self {
        let router = MessageRouter::new();

        let app = Self {
            router,
            temp_endpoints: Vec::new(),
            count: Arc::new(AtomicU64::new(0)),
        };

        app
    }
}

/// A message payload for temperature
#[allow(unused)]
#[derive(Debug)]
struct TempMessage {
    sensor_id: u64,
    temp: f32,
}
impl Payload for TempMessage {}

#[allow(unused)]
#[derive(Debug)]
struct HumidityMessage {
    sensor_id: u64,
    humidity: f32,
}
impl Payload for HumidityMessage {}

#[allow(unused)]
#[derive(Debug)]
struct Task(String);

fn main() {
    let mut app = App::new();

    // Create 100 endpoints handling TempMessage messages
    for i in 0..100 {
        let _count = app.count.clone();
        let endpoint = app
            .router
            .create_endpoint::<TempMessage>()
            .message(move |_msg| {
                _count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                //println!("Received message in callback {msg:?} {x}");
                Task(format!("Returning a task from the closure {i}"))
            });

        app.temp_endpoints.push(endpoint);
    }

    // Create a single HumidityMessage handler
    let _count = app.count.clone();
    let _humidity_handler = app
        .router
        .create_endpoint::<HumidityMessage>()
        .message(move |_msg| {
            _count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            //println!("Got a humidity message {msg:?} {x}");
            Task("set the fan..".into())
        });

    // Send some messages
    loop {
        let _tasks = app.router.handle_message(&mut Message::new(TempMessage {
            sensor_id: 2,
            temp: 21.22,
        }));

        //println!("Tasks in response to TempMessage: {tasks:?}");

        let _tasks = app
            .router
            .handle_message(&mut Message::new(HumidityMessage {
                sensor_id: 3,
                humidity: 86.9,
            }));

        //println!("Tasks in response to HumidityMessage: {tasks:#?}");

        //sleep(Duration::from_millis(10));

        println!("Count {:?}", app.count);
    }
}
