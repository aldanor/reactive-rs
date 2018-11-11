#![crate_type = "lib"]

#[cfg(any(test, feature = "slice-deque"))]
extern crate slice_deque;

pub use stream::{Broadcast, ContextBroadcast, Stream};

mod stream;
