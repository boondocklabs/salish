//! Example of application message routing

use std::{
    sync::{atomic::AtomicU64, Arc},
    time::Instant,
    //thread::sleep,
    //time::Duration,
};

use colored::Colorize as _;
use salish::{endpoint::Endpoint, message::Message, router::MessageRouter, traits::Payload};

/// Example App struct representing some application state
#[derive(Debug)]
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
struct Task(&'static str);

fn main() {
    let mut app = App::new();

    // Create endpoints handling TempMessage messages
    for _i in 0..100000 {
        let _count = app.count.clone();
        let endpoint = app
            .router
            .create_endpoint::<TempMessage>()
            .message(move |_msg| {
                let _tid = std::thread::current().id();
                //println!("{:?}", tid);
                _count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                //println!("Received message in callback {msg:?} handler {_i}");
                //Task(format!("Returning a task from the closure {i}"))
                //Task(format!("Return task {} {}", msg.sensor_id, msg.temp))
                Task("received")
            });

        app.temp_endpoints.push(endpoint);
    }

    println!("{:#?}", app.router);

    /*
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
    */

    let start_time = Instant::now();
    let mut last_time = start_time;
    let mut last_count = 0;

    // Send some messages
    loop {
        let _tasks = app.router.handle_message(&mut Message::new_to(
            salish::message::Destination::Any,
            TempMessage {
                sensor_id: 2,
                temp: 21.22,
            },
        ));

        let count = app.count.load(std::sync::atomic::Ordering::Relaxed);

        if count % 10000000u64 == 0 && count > 0 {
            // Calculate messages per second
            let elapsed = last_time.elapsed().as_secs_f64();
            let messages_per_second = (count - last_count) as f64 / elapsed;

            println!(
                "Messages Processed: {} {}{}",
                app.count.load(std::sync::atomic::Ordering::Relaxed),
                format!("{}", messages_per_second as usize).cyan(),
                "/sec".cyan()
            );

            last_count = count;
            last_time = Instant::now();
        }
    }
}
