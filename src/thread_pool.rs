use crate::constants::{MAX_QUEUE, NO_OF_THREADS};
use crate::shred::{validate_shred, Shred};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Raw pointer wrapper so it can be sent across threads.
// SAFETY: shreds are owned by ShredSet in main and outlive all tp_wait() calls.
struct SendShred(*const Shred);
unsafe impl Send for SendShred {}

struct PoolState {
    queue: VecDeque<SendShred>,
    pending: usize,
    shutdown: bool,
}

pub struct ThreadPool {
    inner: Arc<(Mutex<PoolState>, Condvar, Condvar)>, // (state, work_cond, done_cond)
    handles: Vec<thread::JoinHandle<()>>,
}

impl ThreadPool {
    /// Equivalent to tp_init + tp_start in the C version.
    pub fn new() -> Self {
        let state = PoolState {
            queue: VecDeque::with_capacity(MAX_QUEUE),
            pending: 0,
            shutdown: false,
        };
        let inner = Arc::new((Mutex::new(state), Condvar::new(), Condvar::new()));
        let mut handles = Vec::with_capacity(NO_OF_THREADS);

        for _ in 0..NO_OF_THREADS {
            let inner = Arc::clone(&inner);
            let handle = thread::spawn(move || {
                let (lock, work_cond, done_cond) = &*inner;
                loop {
                    let mut guard = lock.lock().unwrap();

                    while guard.queue.is_empty() && !guard.shutdown {
                        guard = work_cond.wait(guard).unwrap();
                    }

                    if guard.shutdown && guard.queue.is_empty() {
                        break;
                    }

                    let send_shred = guard.queue.pop_front().unwrap();
                    drop(guard);

                    let s = unsafe { &*send_shred.0 };
                    if validate_shred(s) {
                        println!("shred OK     index={}", s.index);
                    } else {
                        println!("shred CORRUPT index={}", s.index);
                    }

                    let mut guard = lock.lock().unwrap();
                    guard.pending -= 1;
                    if guard.pending == 0 {
                        done_cond.notify_all();
                    }
                }
            });
            handles.push(handle);
        }

        ThreadPool { inner, handles }
    }

    /// Equivalent to tp_submit — enqueue a shred for validation.
    pub fn submit(&self, s: *const Shred) {
        let (lock, work_cond, _) = &*self.inner;
        let mut guard = lock.lock().unwrap();
        guard.queue.push_back(SendShred(s));
        guard.pending += 1;
        work_cond.notify_one();
    }

    /// Equivalent to tp_wait — block until all submitted shreds are validated.
    pub fn wait(&self) {
        let (lock, _, done_cond) = &*self.inner;
        let mut guard = lock.lock().unwrap();
        while guard.pending > 0 {
            guard = done_cond.wait(guard).unwrap();
        }
    }

    /// Equivalent to tp_shutdown — signal workers to stop and join threads.
    pub fn shutdown(mut self) {
        let (lock, work_cond, _) = &*self.inner;
        {
            let mut guard = lock.lock().unwrap();
            guard.shutdown = true;
        }
        work_cond.notify_all();
        for handle in self.handles.drain(..) {
            handle.join().unwrap();
        }
    }
}
