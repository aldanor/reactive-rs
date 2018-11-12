//! This crate provides the building blocks for functional reactive programming (FRP)
//! in Rust. It is inspired by
//! [carboxyl](https://crates.io/crates/carboxyl),
//! [frappe](https://crates.io/crates/frappe)
//! and [bidule](https://crates.io/crates/bidule) crates, and
//! various [ReactiveX](http://reactivex.io/) implementations.
//!
//! # Overview
//!
//! ## Purpose
//!
//! The main use case of this library is to simplify creating efficient
//! computational DAGs (or computational trees, to be precise) that operate
//! on streams of values. It does not aim to replicate the entire galaxy of
//! ReactiveX operators, nor does it attempt to delve into
//! futures/concurrency territory.
//!
//! What is a computational tree? First, there's the root at the top, that's
//! where the input values get fed into continuously. Then, we perform
//! computations on these values – each of which may yield zero,
//! one or more values that are sent further down. Some downstream
//! nodes may share their parents – for instance, `g(f(x))` and `h(f(x))`, where `x` is
//! the input and `f` is the intermediate transformation; in this case, we want
//! to make sure we don't have to recompute `f(x)` twice. Moreover, this
//! being Rust, we'd like to ensure we're not copying and cloning any values
//! needlessly, and we generally prefer things to be zero-cost/inlineable
//! when possible. Finally, there are leaves – these are observers, functions
//! that receive transform values and do something with them, likely recording
//! them somewhere or mutating the environment in some other way.
//!
//! ## Terminology
//!
//! - *Observer* - a function that accepts a value and returns nothing (it will
//!   most often that note mutate the external environment in some way).
//! - *Stream* - an object can be subscribed to by passing an observer to it.
//!   Subscribing to a stream consumes the stream, thus at most one observer
//!   can ever be attached to a given stream.
//! - *Broadcast* - an observer that owns a collection of other observers and
//!   transmits its input to each one of them sequentially. A broadcast can
//!   produce new streams, *subscriptions*, each one receiving the same input
//!   as the broadcast itself. Subscription is a stream that adds its
//!   subscribers to the broadcast's collection when being subscribed to.
//!
//! ## Context
//!
//! Streams, broadcasts and observers in this crate operate on pairs of
//! values: the *context* and the *element*. Context can be viewed as
//! optional metadata attached to the original value. Closures required in
//! methods like `.map()` only take one argument (the element) and are
//! expected to return a single value; this way, the element can be changed
//! without touching the context. This can be extremely convenient if you
//! need to access the original input value (or any "upstream" value) way
//! down the computation chain – this way you don't have to propagate
//! it explicitly.
//!
//! Most stream/broadcast methods have an alternative "full" version that
//! operates on both context/element, with `_ctx` suffix.
//!
//! ## Examples
//!
//! Consider the following problem: we have an incoming stream of
//! buy/sell price pairs, and for each incoming event we would like to
//! compute how the current mid-price (the average between the two)
//! compares relatively to the minimum buy price and the maximum sell
//! price over the last three observations. Moreover, we would like to
//! skip the first few events in order to allow the buffer to fill up.
//!
//! Here's one way we could do it (not the most ultimately efficient
//! way of solving this particular problem, but it serves quite well
//! to demonstrate the basic functionality of the crate):
//!
//! ```
//! use std::cell::Cell;
//! use std::f64;
//! use reactive_rs::*;
//!
//! let min_rel = Cell::new(0.);
//! let max_rel = Cell::new(0.);
//!
//! // create a broadcast of (buy, sell) pairs
//! let quotes = SimpleBroadcast::new();
//!
//! // clone the broadcast so we can feed values to it later
//! let last = quotes.clone()
//!     // save the mid-price for later use
//!     .with_ctx_map(|_, &(buy, sell)| (buy + sell) / 2.)
//!     // cache the last three observations
//!     .last_n(3)
//!     // wait until the queue fills up
//!     .filter(|quotes| quotes.len() > 2)
//!     // share the output (slices of values)
//!     .broadcast();
//!
//!// subscribe to the stream of slices
//! let min = last.clone()
//!     // compute min buy price
//!     .map(|p| p.iter().map(|q| q.0).fold(1./0., f64::min));
//!// subscribe to the stream of slices
//! let max = last.clone()
//!     // compute max sell price
//!     .map(|p| p.iter().map(|q| q.1).fold(-1./0., f64::max));
//!
//! // finally, attach observers
//! min.subscribe_ctx(|p, min| min_rel.set(min / p));
//! max.subscribe_ctx(|p, max| max_rel.set(max / p));
//!
//! quotes.send((100., 102.));
//! quotes.send((101., 103.));
//! assert_eq!((min_rel.get(), max_rel.get()), (0., 0.));
//! quotes.send((99., 101.));
//! assert_eq!((min_rel.get(), max_rel.get()), (0.99, 1.03));
//! quotes.send((97., 103.));
//! assert_eq!((min_rel.get(), max_rel.get()), (0.97, 1.03));
//! ```
//!
//! # Lifetimes
//!
//! Many `Stream` trait methods accept mutable closures; observers are
//! also essentially just closures, and they are the only way you can
//! get results from the stream out into the environment. Rest assured,
//! at some point you'll run into lifetime problems (this being Rust,
//! it's pretty much certain).
//!
//! Here's the main rule: lifetimes of observers (that is, lifetimes of
//! what they capture, if anything) may not be shorter than the lifetime of
//! the stream object. Same applies to lifetimes of closures in methods
//! like `.map()`.
//!
//! In some situations it' tough to prove to the compiler you're doing
//! something sane, in which case arena-based allocators (like
//! [`typed-arena`](https://crates.io/crates/typed-arena)) may be
//! of great help – allowing you to tie lifetimes of a bunch of
//! objects together, ensuring simultaneous deallocation.

#![crate_type = "lib"]
#![warn(missing_docs)]

#[cfg(any(test, feature = "slice-deque"))]
extern crate slice_deque;

pub use stream::{Broadcast, SimpleBroadcast, Stream};

mod stream;
