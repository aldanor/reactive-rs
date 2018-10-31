#![crate_type = "lib"]

extern crate slice_deque;

pub use stream::{Broadcast, Stream, StreamExt};

mod stream;
