use crate::{message::Message, traits::internal::SalishMessageInternal as _};

#[allow(unused)]
#[derive(Clone, Debug)]
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
