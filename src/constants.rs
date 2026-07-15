pub const SHRED_SIZE: usize = 128; // instead of 1228

// 64 shreads where loosing up to half of the packets in a set is still recoverable , we are mirroring the ratio of 1:1 but at 1/4 scale
pub const DATA_SHRED: usize = 8; // instead of 32 data shreds 
pub const CODE_SHRED: usize = 8; // instead of 32 code shreds

// its the same erasure coding principle , just shrunken down to loop through and print in demo program

pub const NO_OF_THREADS: usize = 4; // 4/16 (real solana validator arent organized around a 4 thread /16 slot queue )
pub const MAX_QUEUE: usize = 16; // how many local pthreads do I want validating my 16 shreds' CRC32 checksums in parallel
