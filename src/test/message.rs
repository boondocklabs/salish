use crate::{
    message::Message,
    traits::{internal::SalishMessageInternal as _, Payload},
};

#[allow(unused)]
#[derive(Debug)]
enum PayloadA {
    Foo(u64),
    Bar,
}

impl Payload for PayloadA {}

impl<'b> From<&'b Message> for &'b PayloadA {
    fn from(value: &'b Message) -> Self {
        value.inner::<PayloadA>().unwrap()
    }
}

#[allow(unused)]
#[derive(Debug)]
enum PayloadB {
    Baz(u64),
    Foof,
}

impl Payload for PayloadB {}

impl<'b> From<&'b Message> for &'b PayloadB {
    fn from(value: &'b Message) -> Self {
        value.inner::<PayloadB>().unwrap()
    }
}

#[test]
fn simple() {
    let msga = Message::new(PayloadA::Foo(123456));
    let msgb = Message::new(PayloadB::Baz(5678));

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
fn into_ref() {
    let msg = Message::new(PayloadA::Foo(123456));
    let a: &PayloadA = (&msg).into();
    if let PayloadA::Foo(val) = a {
        assert_eq!(*val, 123456)
    }
}
