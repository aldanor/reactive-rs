extern crate reactive_rs;

use std::cell::Cell;

use reactive_rs::{Broadcast, Stream};

#[test]
fn simple_subscribe() {
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

#[test]
fn branches_and_map() {
    let x0 = Cell::new(0);
    let x1 = Cell::new(1);
    let x2 = Cell::new(2);
    let x3 = Cell::new(3);
    let x4 = Cell::new(4);
    let s = Broadcast::<i32>::new();
    s.clone().subscribe(|x| { x0.replace(*x); });
    let t = s.clone().map(|x| x * 2).broadcast();
    t.clone().map(|x| x + 10).subscribe(|x| { x1.replace(*x); });
    t.clone().map(|x| x + 20).subscribe(|x| { x2.replace(*x); });
    let u = t.map(|x| -x).broadcast();
    u.clone().subscribe(|x| { x3.replace(*x); });
    u.map(|x| x - 1).subscribe(|x| { x4.replace(*x); });
    let x = || vec![x0.get(), x1.get(), x2.get(), x3.get(), x4.get()];
    assert_eq!(x(), &[0, 1, 2, 3, 4]);
    s.send(1);
    assert_eq!(x(), &[1, 12, 22, -2, -3]);
    s.send(-5);
    assert_eq!(x(), &[-5, 0, 10, 10, 9]);
}

#[test]
fn filter_and_filter_map() {
    let x0 = Cell::new(0);
    let x1 = Cell::new(0);
    let s = Broadcast::<i32>::new();
    let t = s.clone().filter(|x| x % 5 != 0).broadcast();
    t.clone()
        .filter_map(|x| if x % 2 == 0 { Some(x * 10) } else { None })
        .subscribe(|x| { x0.replace(*x); });
    t.filter(|x| x % 3 != 0).subscribe(|x| { x1.replace(*x); });
    let x = || vec![x0.get(), x1.get()];
    assert_eq!(x(), &[0, 0]);
    s.send(0);
    assert_eq!(x(), &[0, 0]);
    s.send(1);
    assert_eq!(x(), &[0, 1]);
    s.send(2);
    assert_eq!(x(), &[20, 2]);
    s.send(2);
    assert_eq!(x(), &[20, 2]);
    s.send(3);
    assert_eq!(x(), &[20, 2]);
    s.send(6);
    assert_eq!(x(), &[60, 2]);
    s.send(7);
    assert_eq!(x(), &[60, 7]);
}
