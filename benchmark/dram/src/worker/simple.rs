use arrayvec::ArrayVec;

use super::{RateLimiter, Worker};
use crate::{MemBlock, MemSegment, MoveMentMode, Result};

const BATCH_SIZE: usize = 32;

#[repr(align(64))]
pub struct SimpleWorker {
    lcore_id: u16,
    node_id: u16,
    peer_node_id: u16,
    nic: NetworkCard,
    app: Application,
    batch: ArrayVec<MemBlock, BATCH_SIZE>,
}

unsafe impl Send for SimpleWorker {}

impl Worker for SimpleWorker {
    fn name(&self) -> String {
        format!(
            "SW({}-{}:{})",
            self.node_id, self.peer_node_id, self.lcore_id
        )
    }

    fn init(&mut self) -> Result<()> {
        Ok(())
    }

    fn deinit(&mut self) -> Result<()> {
        Ok(())
    }

    fn run_once(&mut self) -> Result<()> {
        let now = rtsc_time::Instant::now();
        self.nic.poll(now.raw(), &mut self.batch);
        if !self.batch.is_empty() {
            self.app.do_recv(&mut self.batch);
        }
        Ok(())
    }
}

impl SimpleWorker {
    pub fn new(
        lcore_id: u16,
        bandwidth: u32,
        num_elems: u32,
        elem_size: u32,
        node_id: u16,
        peer_node_id: u16,
        pkt_size_gen: Box<dyn FnMut() -> u16>,
        mode: MoveMentMode,
    ) -> Box<dyn Worker> {
        let nic = NetworkCard {
            random: if mode == MoveMentMode::R2R || mode == MoveMentMode::R2S {
                true
            } else {
                false
            },
            order: 0,
            seg: MemSegment::new(node_id, num_elems as usize, elem_size as usize).unwrap(),
            generate: pkt_size_gen,
            rate_limit: RateLimiter::new(bandwidth),
        };

        let app = Application {
            random: if mode == MoveMentMode::R2R || mode == MoveMentMode::S2R {
                true
            } else {
                false
            },
            order: 0,
            seg: MemSegment::new(peer_node_id, num_elems as usize, elem_size as usize).unwrap(),
            buf_size: elem_size as usize,
        };

        Box::new(Self {
            lcore_id,
            node_id,
            peer_node_id,
            nic,
            app,
            batch: ArrayVec::new(),
        })
    }
}

/// Rate limiter implemented based on the token bucket algorithm

struct NetworkCard {
    random: bool,
    order: usize,
    seg: MemSegment,
    generate: Box<dyn FnMut() -> u16>,
    rate_limit: RateLimiter,
}

impl NetworkCard {
    pub fn poll(&mut self, tsc: u64, batch: &mut ArrayVec<MemBlock, BATCH_SIZE>) {
        let mut batch_bytes: ArrayVec<u16, BATCH_SIZE> = ArrayVec::new();
        loop {
            if batch_bytes.is_full() {
                break;
            }
            let pkt_size = (self.generate)();
            if self.rate_limit.can_send(tsc, pkt_size as u64) {
                batch_bytes.push(pkt_size);
            } else {
                break;
            }
        }

        for pkt_size in batch_bytes.drain(..) {
            batch.push(if self.random {
                self.seg.random_alloc(pkt_size as usize)
            } else {
                self.seg.sequence_alloc(self.order, pkt_size as usize)
            });
            self.order = self.order.wrapping_add(1);
        }
    }
}

struct Application {
    random: bool,
    order: usize,
    seg: MemSegment,
    buf_size: usize,
}

impl Application {
    pub fn do_recv(&mut self, batch: &mut ArrayVec<MemBlock, BATCH_SIZE>) {
        for block in batch.drain(..) {
            let target_block = if self.random {
                self.seg.random_alloc(self.buf_size)
            } else {
                self.seg.sequence_alloc(self.order, self.buf_size)
            };
            block.move_to(&target_block);
            self.order = self.order.wrapping_add(1);
        }
    }
}
