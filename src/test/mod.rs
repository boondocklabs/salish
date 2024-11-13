mod endpoint;
mod filter;
mod handler;
mod message;
mod router;

/// Payload used for tests
#[allow(unused)]
#[derive(Clone, Debug)]
enum TestPayload {
    Integer(u64),
    String(&'static str),
}

/*
impl<'b> From<&'b Message> for &'b TestPayload {
    fn from(value: &'b Message) -> Self {
        value.inner::<TestPayload>().unwrap()
    }
}
*/
