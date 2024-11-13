use tracing_test::traced_test;

use crate::{
    filter::{Filter, SourceFilter},
    Message,
};

#[derive(Debug, Hash, Clone, Copy)]
enum TestSource {
    Int(i32),
    Unsigned(u64),
    String(&'static str),
}

#[traced_test]
#[test]
fn filter_source() {
    let filter = SourceFilter::default()
        .add(TestSource::Int(1234))
        .add(TestSource::String("pass"))
        .add(TestSource::Unsigned(5656));

    let message = Message::unicast("foo").with_source(TestSource::Int(1234));
    let result = filter.filter(&message);
    assert!(result == true);

    let message = Message::unicast("foo").with_source(TestSource::String("pass"));
    let result = filter.filter(&message);
    assert!(result == true);

    let message = Message::unicast("foo").with_source(TestSource::Unsigned(5656));
    let result = filter.filter(&message);
    assert!(result == true);

    // These messages should not pass the filter
    let message = Message::unicast("foo").with_source(TestSource::Int(999));
    let result = filter.filter(&message);
    assert!(result == false);

    let message = Message::unicast("foo").with_source(TestSource::String("fail"));
    let result = filter.filter(&message);
    assert!(result == false);

    let message = Message::unicast("foo").with_source(TestSource::Unsigned(1234));
    let result = filter.filter(&message);
    assert!(result == false);
}
