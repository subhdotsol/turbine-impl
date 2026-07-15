use crate::constants::{CODE_SHRED, DATA_SHRED, SHRED_SIZE};
use crate::transaction::Transaction;
use crc32fast::Hasher;
use reed_solomon_erasure::galois_8::ReedSolomon;

pub const SHRED_DATA: u8 = 0;
pub const SHRED_CODING: u8 = 1;

#[derive(Copy, Clone)]
pub struct Shred {
    pub data: [u8; SHRED_SIZE],
    pub index: u32,
    pub shred_type: u8,
    pub checksum: u32,
}

pub struct ShredSet {
    pub data_shred: [Shred; DATA_SHRED],
    pub coding_shred: [Shred; CODE_SHRED],
}

fn crc32(data: &[u8]) -> u32 {
    let mut h = Hasher::new();
    h.update(data);
    h.finalize()
}

pub fn shred(tx: &Transaction, set: &mut ShredSet) {
    for i in 0..DATA_SHRED {
        let src = &tx.raw_bytes[i * SHRED_SIZE..(i + 1) * SHRED_SIZE];
        set.data_shred[i].data.copy_from_slice(src);
        set.data_shred[i].index = i as u32;
        set.data_shred[i].shred_type = SHRED_DATA;
        set.data_shred[i].checksum = crc32(&set.data_shred[i].data);
    }
}

pub fn generate_coding_shred(set: &mut ShredSet) {
    let r = ReedSolomon::new(DATA_SHRED, CODE_SHRED).unwrap();

    let mut data: Vec<Vec<u8>> = set.data_shred.iter().map(|s| s.data.to_vec()).collect();
    let mut coding: Vec<Vec<u8>> = vec![vec![0u8; SHRED_SIZE]; CODE_SHRED];

    r.encode_sep(&data, &mut coding).unwrap();

    for i in 0..CODE_SHRED {
        set.coding_shred[i].data.copy_from_slice(&coding[i]);
        set.coding_shred[i].index = i as u32;
        set.coding_shred[i].shred_type = SHRED_CODING;
        set.coding_shred[i].checksum = crc32(&set.coding_shred[i].data);
    }

    // silence unused warning from borrow
    let _ = &mut data;
}

pub fn validate_shred(s: &Shred) -> bool {
    crc32(&s.data) == s.checksum
}
