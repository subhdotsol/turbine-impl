# turbine

A simplified, single-machine simulation of Solana's Turbine shredding protocol, written in Rust.

It takes raw Solana transaction bytes, splits them into fixed-size **data shreds**, generates
**Reed-Solomon coding (parity) shreds**, validates every shred's integrity in parallel across a
thread pool, then deliberately destroys one shred per transaction and reconstructs it from the
remaining shreds — proving the erasure-coding math actually works.

> **Scope note:** this project implements Turbine's *encoding and recovery* math. It does **not**
> implement Turbine's *network propagation* (the deterministic per-shred tree, UDP transport,
> leader signatures, repair/gossip, or equivocation detection). See [What this is / isn't](#what-this-is--isnt) below.

---

## How it works

<img width="1098" height="498" alt="Screenshot 2026-07-14 at 9 13 18 PM" src="https://github.com/user-attachments/assets/fc5ac33e-37f5-4fc5-af73-800788f8df8a" />


At the end, the program prints:

```
total:     10000
recovered: 10000
failed:    0
```

## File structure

```
turbine/
├── Cargo.toml
├── Cargo.lock
├── tx.sh                          # fetches real Solana tx data from an RPC (optional)
├── data/
│   └── transactions.json          # input: JSON array of base64-encoded transactions
├── src/
│   ├── constants.rs               # SHRED_SIZE, DATA_SHRED, CODE_SHRED, thread/queue sizes
│   ├── transaction.rs             # raw decoded-transaction struct
│   ├── shred.rs                   # shredding, coding-shred generation, CRC32 validation
│   ├── thread_pool.rs             # fixed-size thread pool for parallel validation
│   └── main.rs                    # orchestrates the full pipeline
└── wrapper/
    └── rs_wrapper.rs              # Rust interface to the Reed-Solomon library
```

## Key constants

| Constant | Value | Meaning |
|---|---|---|
| `SHRED_SIZE` | 128 bytes | size of one shred's payload |
| `DATA_SHRED` | 8 | data shreds per transaction |
| `CODE_SHRED` | 8 | parity shreds per transaction (1:1 with data shreds) |
| `NO_OF_THREADS` | 4 | worker threads validating shreds in parallel |
| `MAX_QUEUE` | 16 | thread pool queue capacity (exactly 8+8 shreds) |

These are demo-scaled: real Solana shreds are ~1228 bytes and grouped into FEC sets of 32 data +
32 coding shreds. The 1:1 data-to-coding ratio here mirrors the real protocol's tolerance for
losing up to half a set, just at a much smaller size for readability.

## Building & running

```bash
cargo run
```

Requires Rust and Cargo (install via [rustup](https://rustup.rs)).

### Generating input data

`data/transactions.json` can be built from real, live Solana transactions:

```bash
./tx.sh
```

This calls a Solana RPC endpoint's `getBlock`, walks backward through recent slots, and collects
base64-encoded transactions into `data/transactions.json` until it has 10,000. Edit the `RPC`
variable in `tx.sh` to point at your own endpoint before running.

## What this is / isn't

Turbine in production Solana has two jobs: **(1)** encode a block into data + coding shreds, and
**(2)** propagate those shreds through a deterministic, per-shred tree to every validator on the
network. This project implements only **(1)**.

| Real Turbine concept | Implemented here? |
|---|---|
| Data shreds carrying serialized entry data | ✅ |
| Coding shreds carrying pure Reed-Solomon parity | ✅ |
| FEC-set grouping (lose half, still recover) | ✅ (at reduced 8+8 scale) |
| Parallel/local integrity checking | ✅ (via CRC32 + thread pool) |
| Deterministic per-shred tree (`seed = leader_id + slot + shred_index + shred_type`) | ❌ |
| Root rotation per shred | ❌ |
| Leader signature verification | ❌ (CRC32 checks integrity, not authenticity) |
| Network transport (UDP) | ❌ |
| Repair / gossip for missing shreds | ❌ |
| Equivocation detection | ❌ |
| Blockstore / Bank / Status Cache / Tower | ❌ |
| Transaction replay via the SVM | ❌ (bytes are treated as opaque, never executed) |

In short: this is the mathematical core Turbine's data-availability guarantee is built on, run
on one machine, without the networking layer that makes it "Turbine" in the full protocol sense.

## Credits

- [leopard](https://github.com/catid/leopard) by Christopher A. Taylor — Reed-Solomon erasure
  coding, vendored in `thirdparty/leopard/`
