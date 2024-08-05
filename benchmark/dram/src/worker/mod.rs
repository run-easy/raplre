use std::sync::{atomic::AtomicBool, Arc};

use errore::kind::{INTERNAL_ERR, NOT_ALLOWED};

use crate::{throw_rerr, Result};

mod simple;
pub use simple::*;

pub trait Worker: Send {
    fn name(&self) -> String;
    fn init(&mut self) -> Result<()>;
    fn deinit(&mut self) -> Result<()>;
    fn run_once(&mut self) -> Result<()>;
}

pub struct WorkerGroup {
    workers: Vec<Box<dyn Worker>>,
}

impl WorkerGroup {
    pub fn new() -> Self {
        Self {
            workers: Vec::new(),
        }
    }

    pub fn add_worker(&mut self, worker: Box<dyn Worker>) {
        self.workers.push(worker);
    }

    pub fn start(self) -> Result<(WorkerGroupJoinHandle, WorkerGroupCancelHandle)> {
        let mut this = self;
        if this.workers.is_empty() {
            throw_rerr!(NOT_ALLOWED, "No worker to start");
        }

        if this.workers.len() > u16::MAX as usize {
            throw_rerr!(NOT_ALLOWED, "Too many workers");
        }

        let running = Arc::new(AtomicBool::new(true));
        let (tx, rx) = std::sync::mpsc::sync_channel::<usize>(this.workers.len());

        let mut thrs = Vec::new();
        for (worker_id, worker) in this.workers.drain(..).enumerate() {
            let running = running.clone();
            let tx = tx.clone();
            thrs.push(std::thread::spawn(move || {
                Self::main_loop(worker_id, worker, running.clone(), tx.clone())
            }));
        }

        Ok((
            WorkerGroupJoinHandle {
                rx,
                thrs: thrs.into_iter().map(Some).collect(),
            },
            WorkerGroupCancelHandle { running },
        ))
    }

    fn main_loop(
        worker_id: usize,
        worker: Box<dyn Worker>,
        running: Arc<AtomicBool>,
        tx: std::sync::mpsc::SyncSender<usize>,
    ) -> Result<()> {
        let mut worker = worker;
        let worker_name = worker.name();
        let mut errono = None;

        log::info!("Initializing worker {}", worker_name);
        if let Err(e) = worker.init() {
            log::error!("Failed to init worker {}: {}", worker_name, e);
            tx.send(worker_id).unwrap();
            return Err(e);
        } else {
            log::info!("Worker {} initialized!", worker_name);
        }

        log::info!("Worker {} started!", worker_name);
        loop {
            if !running.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }
            if let Err(e) = worker.run_once() {
                log::error!(
                    "An exception `{}` occurs when worker {} running",
                    e,
                    worker_name
                );
                errono = Some(e);
                break;
            }
        }

        log::info!("Deinitializing worker {}", worker_name);
        if let Err(e) = worker.deinit() {
            log::error!("Failed to deinit worker {}: {}", worker_name, e);
            if errono.is_none() {
                tx.send(worker_id).unwrap();
                return Err(e);
            }
        } else {
            log::info!("Worker {} deinitialized!", worker_name);
        }

        if let Some(e) = errono {
            log::error!("Worker {} stopped with error: {}", worker_name, e);
            tx.send(worker_id).unwrap();
            return Err(e);
        }

        tx.send(worker_id).unwrap();
        log::info!("Worker {} stopped!", worker_name);
        Ok(())
    }
}

pub struct WorkerGroupJoinHandle {
    rx: std::sync::mpsc::Receiver<usize>,
    thrs: Vec<Option<std::thread::JoinHandle<Result<()>>>>,
}

unsafe impl Send for WorkerGroupJoinHandle {}

impl WorkerGroupJoinHandle {
    pub fn join(self) -> Result<()> {
        let this = self;
        let mut num_workers = this.thrs.len();
        let mut thrs = this.thrs;
        loop {
            if num_workers == 0 {
                break;
            }

            match this.rx.recv() {
                Ok(worker_id) => {
                    thrs[worker_id].take().unwrap().join().unwrap()?;
                    num_workers -= 1;
                }
                Err(e) => {
                    log::error!("Failed to receive worker status: {}", e);
                    throw_rerr!(INTERNAL_ERR, "Failed to receive worker status");
                }
            }
        }

        Ok(())
    }
}

pub struct WorkerGroupCancelHandle {
    running: Arc<AtomicBool>,
}

unsafe impl Send for WorkerGroupCancelHandle {}

impl WorkerGroupCancelHandle {
    pub fn cancel(self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}

pub struct RateLimiter {
    rate: f64,
    /// Scaling factor used to enhance the accuracy of the rate limiter,
    /// with an effective range spanning from 1 Mbps to 100 Gbps
    scale: f64,
    last_tsc: u64,
    capacity: u64,
    available_tokens: u64,
}

impl RateLimiter {
    pub fn new(bandwidth: u32) -> Self {
        const M_BIT_PER_SECOND: f64 = 1000000.0;
        let cpu_freq = rtsc_time::cycles_per_sec();
        let mut bits_per_second = bandwidth as f64 * M_BIT_PER_SECOND as f64;
        let mut scale = 1.0;
        let rate = {
            while bits_per_second < 1000000000000.0 {
                bits_per_second *= 10.0;
                scale *= 10.0;
            }

            bits_per_second as f64 / cpu_freq as f64
        };

        let capacity = 1024 * 2048 * 8 * scale as u64;
        Self {
            rate,
            last_tsc: 0,
            scale: scale,
            capacity: capacity,
            available_tokens: 0,
        }
    }

    #[inline]
    pub fn can_send(&mut self, tsc: u64, tokens: u64) -> bool {
        let tokens = tokens * 8 * self.scale as u64;
        let elapsed_tsc = tsc - self.last_tsc;
        self.last_tsc = tsc;

        let increased_tokens = (elapsed_tsc as f64 * self.rate).round() as u64;

        self.available_tokens = u64::min(self.capacity, increased_tokens + self.available_tokens);

        if self.available_tokens < tokens {
            return false;
        } else {
            self.available_tokens -= tokens;
            return true;
        }
    }
}
