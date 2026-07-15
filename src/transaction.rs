use crate::constants::{DATA_SHRED, SHRED_SIZE};

pub struct Transaction {
    pub raw_bytes: [u8; DATA_SHRED * SHRED_SIZE],
    pub size: u32,
}
