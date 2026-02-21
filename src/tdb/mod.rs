//! Decoder/Encoder for Air Avionics TDB file format.
//!
//! The [decode_file] function can be used to decode FlarmNet files in
//! Air Avionics TDB format.

mod consts;
mod decode;
mod encode;

pub use decode::*;
pub use encode::*;
