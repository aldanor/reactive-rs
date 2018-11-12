reactive-rs
===========

[![Build](https://api.travis-ci.org/aldanor/reactive-rs.svg)](https://travis-ci.org/aldanor/reactive-rs)
[![Crate](http://meritbadge.herokuapp.com/reactive-rs)](https://crates.io/crates/reactive-rs)
[![Docs](https://docs.rs/reactive-rs/badge.svg)](https://docs.rs/reactive-rs)

This crate provides the building blocks for functional reactive programming (FRP)
in Rust. It is inspired by
[carboxyl](https://crates.io/crates/carboxyl),
[frappe](https://crates.io/crates/frappe)
and [bidule](https://crates.io/crates/bidule) crates, and
various [ReactiveX](http://reactivex.io/) implementations.

### Documentation

[docs.rs/reactive-rs](https://docs.rs/reactive-rs)

### Purpose

The main use case of this library is to simplify creating efficient
computational DAGs (or computational trees, to be precise) that operate
on streams of values. It does not aim to replicate the entire galaxy of
ReactiveX operators, nor does it attempt to delve into
futures/concurrency territory.

What is a computational tree? First, there's the root at the top, that's
where the input values get fed into continuously. Then, we perform
computations on these values – each of which may yield zero,
one or more values that are sent further down. Some downstream
nodes may share their parents – for instance, `g(f(x))` and `h(f(x))`, where `x` is
the input and `f` is the intermediate transformation; in this case, we want
to make sure we don't have to recompute `f(x)` twice. Moreover, this
being Rust, we'd like to ensure we're not copying and cloning any values
needlessly, and we generally prefer things to be zero-cost/inlineable
when possible. Finally, there are leaves – these are observers, functions
that receive transform values and do something with them, likely recording
them somewhere or mutating the environment in some other way.

### Context

Streams, broadcasts and observers in this crate operate on pairs of
values: the *context* and the *element*. Context can be viewed as
optional metadata attached to the original value. Closures required in
methods like `.map()` only take one argument (the element) and are
expected to return a single value; this way, the element can be changed
without touching the context. This can be extremely convenient if you
need to access the original input value (or any "upstream" value) way
down the computation chain – this way you don't have to propagate
it explicitly.

Most stream/broadcast methods have an alternative "full" version that
operates on both context/element, with `_ctx` suffix.

### Usage example

Consider the following problem: we have an incoming stream of
buy/sell price pairs, and for each incoming event we would like to
compute how the current mid-price (the average between the two)
compares relatively to the minimum buy price and the maximum sell
price over the last three observations. Moreover, we would like to
skip the first few events in order to allow the buffer to fill up.

Here's one way we could do it (not the most ultimately efficient
way of solving this particular problem, but it serves quite well
to demonstrate the basic functionality of the crate):

```rust
use std::cell::Cell;
use std::f64;
use reactive_rs::*;

let min_rel = Cell::new(0.);
let max_rel = Cell::new(0.);

// create a broadcast of (buy, sell) pairs
let quotes = SimpleBroadcast::new();

// clone the broadcast so we can feed values to it later
let last = quotes.clone()
    // save the mid-price for later use
    .with_ctx_map(|_, &(buy, sell)| (buy + sell) / 2.)
    // cache the last three observations
    .last_n(3)
    // wait until the queue fills up
    .filter(|quotes| quotes.len() > 2)
    // share the output (slices of values)
    .broadcast();

// subscribe to the stream of slices
let min = last.clone()
    // compute min buy price
    .map(|p| p.iter().map(|q| q.0).fold(1./0., f64::min));
// subscribe to the stream of slices
let max = last.clone()
    // compute max sell price
    .map(|p| p.iter().map(|q| q.1).fold(-1./0., f64::max));

// finally, attach observers
min.subscribe_ctx(|p, min| min_rel.set(min / p));
max.subscribe_ctx(|p, max| max_rel.set(max / p));

quotes.send((100., 102.));
quotes.send((101., 103.));
assert_eq!((min_rel.get(), max_rel.get()), (0., 0.));
quotes.send((99., 101.));
assert_eq!((min_rel.get(), max_rel.get()), (0.99, 1.03));
quotes.send((97., 103.));
assert_eq!((min_rel.get(), max_rel.get()), (0.97, 1.03));
```

### License

The MIT License (MIT)

Copyright (c) 2018 Ivan Smirnov

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
