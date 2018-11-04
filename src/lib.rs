#![crate_type = "lib"]

extern crate slice_deque;

pub use stream::{ContextBroadcast, Broadcast, Stream};

mod stream;
