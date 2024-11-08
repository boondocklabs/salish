use crate::traits::Payload;

mod endpoint;
mod handler;
mod message;
mod router;

/// Payload used for tests
#[allow(unused)]
#[derive(Debug)]
enum TestPayload {
    Integer(u64),
    String(&'static str),
}
impl Payload for TestPayload {}

/*
impl<'b> From<&'b Message> for &'b TestPayload {
    fn from(value: &'b Message) -> Self {
        value.inner::<TestPayload>().unwrap()
    }
}
*/
