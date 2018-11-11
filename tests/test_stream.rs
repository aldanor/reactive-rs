extern crate reactive_rs;

use std::cell::{Cell, RefCell};

use reactive_rs::{Broadcast, ContextBroadcast, Stream};

fn subscribe_cell<'a, S>(stream: S, cell: &'a Cell<S::Item>)
where
    S: Stream<'a>,
    S::Item: Sized + Clone,
{
    stream.subscribe(move |x| cell.set(x.clone()));
}

fn subscribe_cell_ctx<'a, S>(stream: S, cell: &'a Cell<(S::Context, S::Item)>)
where
    S: Stream<'a>,
    S::Item: Sized + Clone,
    S::Context: Sized + Clone,
{
    stream.subscribe_ctx(move |ctx, x| cell.set((ctx.clone(), x.clone())));
}

#[test]
fn test_subscribe_no_ctx() {
    let v = Cell::new(0);
    let v_ctx = Cell::new(((), 0));
    let s = Broadcast::<i32>::new();
    subscribe_cell(s.clone(), &v);
    subscribe_cell_ctx(s.clone(), &v_ctx);
    s.send(3);
    assert_eq!(v.get(), 3);
    assert_eq!(v_ctx.get(), ((), 3));
    s.send_ctx((), 4);
    assert_eq!(v.get(), 4);
    assert_eq!(v_ctx.get(), ((), 4));
}

#[test]
fn test_subscribe_explicit_ctx() {
    let v = Cell::new(0);
    let v_ctx = Cell::new((0., 0));
    let s = ContextBroadcast::<f64, i32>::new();
    subscribe_cell(s.clone(), &v);
    subscribe_cell_ctx(s.clone(), &v_ctx);
    s.send(3);
    assert_eq!(v.get(), 3);
    assert_eq!(v_ctx.get(), (0., 3));
    s.send_ctx(3.14, 4);
    assert_eq!(v.get(), 4);
    assert_eq!(v_ctx.get(), (3.14, 4));
}

#[test]
fn test_context_survives_operators() {
    let v = Cell::new(-1);
    let v_ctx = Cell::new((-1., -1));
    let s = ContextBroadcast::<f64, i32>::new();
    let t = s.clone().filter(|x| x % 3 != 0).map(|x| x * 2).broadcast();
    subscribe_cell(t.clone(), &v);
    subscribe_cell_ctx(t.clone(), &v_ctx);
    s.send(0);
    assert_eq!(v.get(), -1);
    assert_eq!(v_ctx.get(), (-1., -1));
    s.send_ctx(1., 2);
    assert_eq!(v.get(), 4);
    assert_eq!(v_ctx.get(), (1., 4));
    s.send(1);
    assert_eq!(v.get(), 2);
    assert_eq!(v_ctx.get(), (0., 2));
}

#[test]
fn test_multiple_broadcast_parents() {
    let v = Cell::new(-1);
    let v_ctx = Cell::new((-1., -1));
    let s = ContextBroadcast::<f64, i32>::new();
    let t = s.clone().filter(|x| x % 3 != 0).map(|x| x * 2).broadcast();
    subscribe_cell(t.clone(), &v);
    subscribe_cell_ctx(t.clone(), &v_ctx);
    s.send(0);
    assert_eq!(v.get(), -1);
    assert_eq!(v_ctx.get(), (-1., -1));
    t.send_ctx(3.14, 2);
    assert_eq!(v.get(), 2);
    assert_eq!(v_ctx.get(), (3.14, 2));
}

#[test]
fn test_ctx_methods() {
    let v1_ctx = Cell::new((-1., -1));
    let v2_ctx = Cell::new((-1., -1));
    let v3_ctx = Cell::new((-1., -1.));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().with_ctx(1.23);
    let s2 = s.clone().with_ctx_map(|ctx, x| *ctx + (*x as f64));
    let s3 = s.clone().ctx();
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    subscribe_cell_ctx(s3, &v3_ctx);
    s.send_ctx(0.1, 2);
    assert_eq!(v1_ctx.get(), (1.23, 2));
    assert_eq!(v2_ctx.get(), (2.1, 2));
    assert_eq!(v3_ctx.get(), (0.1, 0.1));
}

#[test]
fn test_map() {
    let v1_ctx = Cell::new((-1., -1));
    let v2_ctx = Cell::new((-1., -1.));
    let v3_ctx = Cell::new((-1., -1.));
    let v4_ctx = Cell::new((-1., -1.));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().map(|x| x * 2);
    let s2 = s.clone().map_ctx(|ctx, x| *ctx + (*x as f64));
    let s3 = s.clone().map(|x| x * 2).map_ctx(|ctx, x| *ctx + (*x as f64));
    let s4 = s.clone().map_ctx(|ctx, x| *ctx + (*x as f64)).map(|x| x * 2.);
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    subscribe_cell_ctx(s3, &v3_ctx);
    subscribe_cell_ctx(s4, &v4_ctx);
    s.send_ctx(0.1, 2);
    assert_eq!(v1_ctx.get(), (0.1, 4));
    assert_eq!(v2_ctx.get(), (0.1, 2.1));
    assert_eq!(v3_ctx.get(), (0.1, 4.1));
    assert_eq!(v4_ctx.get(), (0.1, 4.2));
    s.send_ctx(-0.2, 3);
    assert_eq!(v1_ctx.get(), (-0.2, 6));
    assert_eq!(v2_ctx.get(), (-0.2, 2.8));
    assert_eq!(v3_ctx.get(), (-0.2, 5.8));
    assert_eq!(v4_ctx.get(), (-0.2, 5.6));
}

#[test]
fn test_map_both() {
    let v1_ctx = Cell::new((-1, -1));
    let v2_ctx = Cell::new((-1, -1.));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().map_both(|x| (x * 2, x * 3));
    let s2 = s.clone().map_both_ctx(|ctx, x| (*x, *ctx));
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    s.send_ctx(0.1, 2);
    assert_eq!(v1_ctx.get(), (4, 6));
    assert_eq!(v2_ctx.get(), (2, 0.1));
}

#[test]
fn test_filter() {
    let v1_ctx = Cell::new((-1., -1));
    let v2_ctx = Cell::new((-1., -1));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().filter(|x| x % 2 == 0);
    let s2 = s.clone().filter_ctx(|ctx, x| (*x as f64) > *ctx);
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    s.send_ctx(4.1, 4);
    assert_eq!(v1_ctx.get(), (4.1, 4));
    assert_eq!(v2_ctx.get(), (-1., -1));
    s.send_ctx(2.9, 3);
    assert_eq!(v1_ctx.get(), (4.1, 4));
    assert_eq!(v2_ctx.get(), (2.9, 3));
}

#[test]
fn test_filter_map() {
    let v1_ctx = Cell::new((-1., -1));
    let v2_ctx = Cell::new((-1., -1.));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().filter_map(|x| if x % 2 == 0 { Some(2 * x) } else { None });
    let s2 =
        s.clone().filter_map_ctx(
            |ctx, x| if (*x as f64) > *ctx { Some(ctx + (*x as f64)) } else { None },
        );
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    s.send_ctx(4.1, 4);
    assert_eq!(v1_ctx.get(), (4.1, 8));
    assert_eq!(v2_ctx.get(), (-1., -1.));
    s.send_ctx(2.9, 3);
    assert_eq!(v1_ctx.get(), (4.1, 8));
    assert_eq!(v2_ctx.get(), (2.9, 5.9));
}

#[test]
fn test_fold() {
    let v1_ctx = Cell::new((-1., -1));
    let v2_ctx = Cell::new((-1., -1.));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().fold(|acc, x| acc + x, 0);
    let s2 = s.clone().fold_ctx(|ctx, acc, x| (acc + ctx) + (*x as f64), 0.);
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    s.send_ctx(4.1, 4);
    assert_eq!(v1_ctx.get(), (4.1, 4));
    assert_eq!(v2_ctx.get(), (4.1, 8.1));
    s.send_ctx(2.9, 3);
    assert_eq!(v1_ctx.get(), (2.9, 7));
    assert_eq!(v2_ctx.get(), (2.9, 14.));
}

#[test]
fn test_inspect() {
    let i1 = Cell::new(-1);
    let i2_ctx = Cell::new((-1., -1));
    let i3 = Cell::new(-1);
    let i4_ctx = Cell::new((-1., -1));
    let v1_ctx = Cell::new((-1., -1));
    let v2_ctx = Cell::new((-1., -1));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().inspect(|x| i1.set(*x));
    let s2 = s.clone().inspect_ctx(|ctx, x| i2_ctx.set((*ctx, *x)));
    let _s3 = s.clone().inspect(|x| i3.set(*x));
    let _s4 = s.clone().inspect_ctx(|ctx, x| i4_ctx.set((*ctx, *x)));
    subscribe_cell_ctx(s1, &v1_ctx);
    subscribe_cell_ctx(s2, &v2_ctx);
    s.send_ctx(1.23, 4);
    assert_eq!(i1.get(), 4);
    assert_eq!(i2_ctx.get(), (1.23, 4));
    assert_eq!(i3.get(), -1);
    assert_eq!(i4_ctx.get(), (-1., -1));
    assert_eq!(v1_ctx.get(), (1.23, 4));
    assert_eq!(v2_ctx.get(), (1.23, 4));
}

#[test]
fn test_last_n() {
    let v1_ctx = RefCell::<(f64, Vec<i32>)>::new((-1., vec![]));
    let s = ContextBroadcast::<f64, i32>::new();
    let s1 = s.clone().last_n(2).map(|x| x.iter().cloned().collect());
    s1.subscribe_ctx(|ctx, x: &Vec<_>| *v1_ctx.borrow_mut() = (*ctx, x.clone()));
    assert_eq!(*v1_ctx.borrow(), (-1., vec![]));
    s.send_ctx(1.23, 4);
    assert_eq!(*v1_ctx.borrow(), (1.23, vec![4]));
    s.send_ctx(2.34, 6);
    assert_eq!(*v1_ctx.borrow(), (2.34, vec![4, 6]));
    s.send_ctx(3.45, 8);
    assert_eq!(*v1_ctx.borrow(), (3.45, vec![6, 8]));
    s.send_ctx(4.56, 10);
    assert_eq!(*v1_ctx.borrow(), (4.56, vec![8, 10]));
}

#[test]
fn test_simple_subscribe() {
    let v = Cell::new(1);
    let s = Broadcast::<i32>::new();
    s.clone().subscribe(|x| v.set(*x + v.get()));
    assert_eq!(v.get(), 1);
    s.send(2);
    assert_eq!(v.get(), 3);
    s.send(&3);
    assert_eq!(v.get(), 6);
    s.feed(-5..-1);
    assert_eq!(v.get(), -8);
}

#[test]
fn test_broadcast_map() {
    let x0 = Cell::new(0);
    let x1 = Cell::new(1);
    let x2 = Cell::new(2);
    let x3 = Cell::new(3);
    let x4 = Cell::new(4);
    let s = Broadcast::<i32>::new();
    s.clone().subscribe(|x| x0.set(*x));
    let t = s.clone().map(|x| x * 2).broadcast();
    t.clone().map(|x| x + 10).subscribe(|x| x1.set(*x));
    t.clone().map(|x| x + 20).subscribe(|x| x2.set(*x));
    let u = t.map(|x| -x).broadcast();
    u.clone().subscribe(|x| x3.set(*x));
    u.map(|x| x - 1).subscribe(|x| x4.set(*x));
    let x = || vec![x0.get(), x1.get(), x2.get(), x3.get(), x4.get()];
    assert_eq!(x(), &[0, 1, 2, 3, 4]);
    s.send(1);
    assert_eq!(x(), &[1, 12, 22, -2, -3]);
    s.send(-5);
    assert_eq!(x(), &[-5, 0, 10, 10, 9]);
}

#[test]
fn test_broadcast_filter() {
    let x0 = Cell::new(0);
    let x1 = Cell::new(0);
    let s = Broadcast::<i32>::new();
    let t = s.clone().filter(|x| x % 5 != 0).broadcast();
    t.clone()
        .filter_map(|x| if x % 2 == 0 { Some(x * 10) } else { None })
        .subscribe(|x| x0.set(*x));
    t.filter(|x| x % 3 != 0).subscribe(|x| x1.set(*x));
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
