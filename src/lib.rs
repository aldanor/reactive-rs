#![crate_type = "lib"]
#![warn(missing_docs)]

#[cfg(any(test, feature = "slice-deque"))]
extern crate slice_deque;

pub use stream::{Broadcast, SimpleBroadcast, Stream};

mod stream;
