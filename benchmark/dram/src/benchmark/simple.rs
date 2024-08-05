use std::collections::HashMap;

use clap::Parser;
use errore::kind::INVALID_VALUE;
use errore::kind::NOT_ALLOWED;
use serde::Serialize;

use crate::throw_rerr;
use crate::PktSizeDistr;
use crate::Result;

use super::BenchmarkCancelHandle;
use super::BenchmarkJoinHandle;
use super::SimpleWorker;
use super::WorkerGroup;

#[derive(Debug, Parser, Serialize)]
pub struct SimpleBenchmark {
    #[arg(long, help = "Enable interleaved mode")]
    interleave: bool,
    #[arg(
        long,
        help = "The size of packet buffers in bytes",
        default_value_t = 2048
    )]
    buf_size: u32,
    #[arg(long, help = "The number of packet buffers", default_value_t = 1024)]
    num_bufs: u32,
    #[arg(long, help = "Maximum bandwidth (Mbps)", default_value_t = 1000)]
    bandwidth: u32,
    #[arg(long, help = "The distribution of packet size",default_value_t = String::from("constant(64)"))]
    pkt_size_distri: String,
    #[arg(long, help = "The number of worker threads", default_value_t = 1)]
    num_workers: u32,
    #[arg(long, help = "The mode of data movement",default_value_t = crate::MoveMentMode::R2R)]
    mode: crate::MoveMentMode,
}

impl SimpleBenchmark {
    pub fn start(&self) -> Result<(BenchmarkJoinHandle, BenchmarkCancelHandle)> {
        if self.num_bufs * self.buf_size == 0 {
            throw_rerr!(
                INVALID_VALUE,
                "Failed to run simple benchmakr,block size cannot be zero"
            );
        }

        if self.bandwidth == 0 {
            log::info!("SimpleBenchmark: Unlimited bandwidth");
        }

        if self.interleave && cpu::topology().max_num_nodes() % 2 != 0 {
            throw_rerr!(
                NOT_ALLOWED,
                "Failed to run simple benchmakr, interleave mode requires even number of nodes"
            );
        }

        let pkt_size_gen = PktSizeDistr::parse(&self.pkt_size_distri)?;

        let mut group = WorkerGroup::new();
        let mut peer_node = HashMap::new();
        let mut node_id = 0;

        let max_nodes = cpu::topology().max_num_nodes();

        loop {
            if node_id >= max_nodes {
                break;
            }
            if self.interleave {
                peer_node.insert(node_id, node_id + 1);
                node_id += 2;
            } else {
                peer_node.insert(node_id, node_id);
                node_id += 1;
            }
        }

        let mut num_workers = self.num_workers;
        for node_id in 0..max_nodes {
            let lcores = cpu::topology().lcores_of_node(node_id).unwrap();
            for (lcore_id, enable) in lcores.into_iter().enumerate() {
                if enable && num_workers > 0 {
                    log::info!(
                        "SimpleBenchmark: Add worker on lcore {} for node pair {}-{}",
                        lcore_id,
                        node_id,
                        peer_node.get(&node_id).unwrap()
                    );
                    group.add_worker(SimpleWorker::new(
                        lcore_id as u16,
                        self.bandwidth,
                        self.num_bufs,
                        self.buf_size,
                        node_id,
                        *peer_node.get(&node_id).unwrap(),
                        pkt_size_gen.generate(),
                        self.mode,
                    ));
                    num_workers -= 1;
                }
            }
        }

        let (join_handle, cancel_handle) = group.start()?;

        std::thread::sleep(std::time::Duration::from_secs(1));

        log::info!(
            "SimpleBenchmark: started {} workers, remaining {} workers",
            self.num_workers - num_workers,
            num_workers
        );

        log::info!(
          "SimpleBenchmakr: total allocated {} memory, the maximum amount of data movement per second is {} Mbit/s",
          (self.num_workers - num_workers) * (self.buf_size * self.num_bufs),
          (self.num_workers - num_workers) * (self.bandwidth)
        );

        Ok((
            BenchmarkJoinHandle {
                join_handle: Box::new(move || join_handle.join()),
            },
            BenchmarkCancelHandle {
                cancel_handle: Box::new(move || cancel_handle.cancel()),
            },
        ))
    }
}
