use std::any::Any;

use crate::{message::Message, traits::internal::SalishMessageInternal as _};

#[allow(unused)]
#[derive(Debug)]
enum PayloadA {
    Foo(u64),
    Bar,
}

impl<'b> From<&'b Message> for &'b PayloadA {
    fn from(value: &'b Message) -> Self {
        value.inner::<PayloadA>().unwrap()
    }
}

#[allow(unused)]
#[derive(Clone, Debug)]
enum PayloadB {
    Baz(u64),
    Foof,
}

#[test]
fn simple() {
    let msga = Message::unicast(PayloadA::Foo(123456));
    let msgb = Message::broadcast(PayloadB::Baz(5678));

    assert!(msga.is_type::<PayloadA>());
    assert!(msgb.is_type::<PayloadB>());

    let a = msga.inner::<PayloadA>().unwrap();
    if let PayloadA::Foo(val) = a {
        assert_eq!(*val, 123456)
    }

    let b = msgb.inner::<PayloadB>().unwrap();
    if let PayloadB::Baz(val) = b {
        assert_eq!(*val, 5678)
    }
}

#[test]
fn owned() {
    let msg = Message::unicast(PayloadA::Foo(123456));
    let payload = msg.into_inner::<PayloadA>();

    assert!(payload.is_some());
}

#[test]
fn into_ref() {
    let msg = Message::unicast(PayloadA::Foo(123456));
    let a: &PayloadA = (&msg).into();
    if let PayloadA::Foo(val) = a {
        assert_eq!(*val, 123456)
    }
}
