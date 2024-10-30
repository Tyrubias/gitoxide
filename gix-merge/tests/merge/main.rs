use gix_hash::ObjectId;
extern crate core;

#[cfg(feature = "blob")]
mod blob;

pub use gix_testtools::Result;

fn hex_to_id(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("40 bytes hex")
}
