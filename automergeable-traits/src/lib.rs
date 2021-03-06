// #![warn(clippy::pedantic)]
#![warn(clippy::nursery)]

mod from;
mod to;

pub use from::{FromAutomerge, FromAutomergeError, Text};
pub use to::ToAutomerge;

/// Overall trait for requiring all automerge sub-traits.
pub trait Automergeable: to::ToAutomerge + from::FromAutomerge {}

impl<T> Automergeable for T where T: to::ToAutomerge + from::FromAutomerge {}
