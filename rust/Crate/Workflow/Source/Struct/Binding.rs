//! The bridge from the browser substrate to the two consumer-owned ports.
//!
//! One module per consumer, because they are not the same port and must not be
//! made to be: [`Page`] serves the chat driver's `Page` - open, ask, release,
//! no ownership - and [`Measure`] serves design verification's `Browse` - read
//! the named elements of one configured page, retaining nothing.

pub mod Measure;
pub mod Page;
