#![crate_type = "lib"]

#[cfg(feature = "slice-deque")]
extern crate slice_deque;

pub use stream::{ContextBroadcast, Broadcast, Stream};

mod stream;
