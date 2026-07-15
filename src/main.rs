mod constants;
mod shred;
mod thread_pool;
mod transaction;

use base64::{engine::general_purpose::STANDARD, Engine as _};
use constants::{CODE_SHRED, DATA_SHRED, SHRED_SIZE};
use reed_solomon_erasure::galois_8::ReedSolomon;
use shred::{generate_coding_shred, shred, ShredSet, Shred, SHRED_DATA, SHRED_CODING};
use thread_pool::ThreadPool;
use transaction::Transaction;

fn main() {
    let json_str = std::fs::read_to_string("data/transactions.json")
        .expect("failed to open data/transactions.json");

    let b64_list: Vec<String> =
        serde_json::from_str(&json_str).expect("failed to parse JSON");

    let total = b64_list.len();
    let mut recovered = 0usize;
    let mut failed = 0usize;

    let tp = ThreadPool::new();

    for b64 in &b64_list {
        let raw = match STANDARD.decode(b64) {
            Ok(r) => r,
            Err(_) => continue,
        };

        let mut tx = Transaction {
            raw_bytes: [0u8; DATA_SHRED * SHRED_SIZE],
            size: 0,
        };
        let copy_len = raw.len().min(tx.raw_bytes.len());
        tx.raw_bytes[..copy_len].copy_from_slice(&raw[..copy_len]);
        tx.size = copy_len as u32;

        let mut set = make_shred_set();
        shred(&tx, &mut set);
        generate_coding_shred(&mut set);

        // corrupt shred[0] on the first transaction only — flip a byte so CRC32 mismatches
        if b64 == b64_list.first().unwrap() {
            set.data_shred[0].data[0] ^= 0xFF;
        }

        for i in 0..DATA_SHRED {
            tp.submit(&set.data_shred[i] as *const Shred);
        }
        for i in 0..CODE_SHRED {
            tp.submit(&set.coding_shred[i] as *const Shred);
        }
        tp.wait();

        // save shred[2], zero it to simulate a lost packet
        let saved = set.data_shred[2].data;
        set.data_shred[2].data = [0u8; SHRED_SIZE];

        // reconstruct with reed-solomon
        let r = ReedSolomon::new(DATA_SHRED, CODE_SHRED).unwrap();
        let mut shards: Vec<Option<Vec<u8>>> = set
            .data_shred
            .iter()
            .enumerate()
            .map(|(i, s)| if i == 2 { None } else { Some(s.data.to_vec()) })
            .collect();
        for s in set.coding_shred.iter() {
            shards.push(Some(s.data.to_vec()));
        }

        match r.reconstruct_data(&mut shards) {
            Ok(_) => {
                let rebuilt = shards[2].as_ref().unwrap();
                if rebuilt.as_slice() == &saved {
                    recovered += 1;
                } else {
                    failed += 1;
                }
            }
            Err(_) => failed += 1,
        }
    }

    tp.shutdown();

    println!("total:     {}", total);
    println!("recovered: {}", recovered);
    println!("failed:    {}", failed);
}

fn make_shred_set() -> ShredSet {
    const ZERO_SHRED: Shred = Shred {
        data: [0u8; SHRED_SIZE],
        index: 0,
        shred_type: SHRED_DATA,
        checksum: 0,
    };
    ShredSet {
        data_shred: [ZERO_SHRED; DATA_SHRED],
        coding_shred: [Shred { shred_type: SHRED_CODING, ..ZERO_SHRED }; CODE_SHRED],
    }
}
