extern crate reactive_rs;

use std::cell::Cell;

use reactive_rs::{Broadcast, Stream};

#[test]
fn test_simple_subscribe() {
    let v = Cell::new(1);
    let s = Broadcast::<i32>::new();
    s.clone().subscribe(|x| { v.replace(*x + v.get()); });
    assert_eq!(v.get(), 1);
    s.send(2);
    assert_eq!(v.get(), 3);
    s.send(&3);
    assert_eq!(v.get(), 6);
    s.feed(-5..-1);
    assert_eq!(v.get(), -8);
}
