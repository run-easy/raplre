#[derive(Debug, Parser, Serialize)]
struct Args {
    #[arg(long, help = "Enable hugepage")]
    huge: bool,
    #[arg(long,help="hugepage size",default_value_t = String::from("2M"))]
    page_size: String,
    #[arg(long, help = "Simulation time in minutes", default_value_t = 15)]
    time: u32,
    #[arg(long, help = "Configuration dump directory")]
    dump_dir: Option<PathBuf>,
    #[arg(long, help = "Configuration dump file name")]
    dump_name: Option<String>,
    #[arg(long, help = "Print statistics", default_value_t = false)]
    stat: bool,
    #[arg(
        long,
        help = "Print statistics interval in second",
        default_value_t = 1
    )]
    interval: u32,
    #[command(subcommand)]
    benchmark: Benchmark,
}

#[derive(Debug, Subcommand, Serialize)]
enum Benchmark {
    /// simple
    Simple(SimpleBenchmark),
}

impl Benchmark {
    fn start(&self) -> Result<(BenchmarkJoinHandle, BenchmarkCancelHandle)> {
        match self {
            Benchmark::Simple(bm) => bm.start(),
        }
    }
}

impl Display for MoveMentMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MoveMentMode::R2R => write!(f, "R2R"),
            MoveMentMode::R2S => write!(f, "R2S"),
            MoveMentMode::S2R => write!(f, "S2R"),
            MoveMentMode::S2S => write!(f, "S2S"),
        }
    }
}

mod error;
use std::{fmt::Display, path::PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use error::*;
mod worker;
use serde::Serialize;
use worker::*;
mod benchmark;
use benchmark::*;
mod block;
use block::*;
mod stat;
use stat::*;

type Result<T> = std::result::Result<T, RError>;

fn main() {
    std::env::set_var("RUST_LOG", "error");
    env_logger::init();
    let args = Args::parse();

    let mut page_size = match parse_page_size_str(&args.page_size) {
        Ok(page_size) => page_size,
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    };

    if page_size == 0 {
        page_size = default_page_size();
    }
    if !supported_page_size(page_size) {
        log::error!("Invalid page size: {}", page_size);
        std::process::exit(1);
    }

    if !rtsc_time::tsc_stable() {
        log::error!(
            "Constant TSC is not supported by the CPU.This benchmark relies on this feature."
        );
        std::process::exit(1);
    }

    block::init(|| GlobalConf {
        page_size: page_size,
        huge_enabled: args.huge,
    });

    if let Some(dump_dir) = args.dump_dir.as_ref() {
        if dump_dir.exists() && !dump_dir.is_dir() {
            log::error!(
                "Path `{}` already exist and not a directory",
                dump_dir.to_str().unwrap()
            );
            std::process::exit(1);
        }

        if args.dump_name.is_none() {
            log::error!("Dump file name not given. Please see '-h' or '--help'.");
            std::process::exit(1);
        }

        if !dump_dir.exists() {
            match std::fs::create_dir_all(dump_dir) {
                Ok(_) => {}
                Err(e) => {
                    log::error!(
                        "Failed to create dump directory `{}` (error:{})",
                        dump_dir.to_str().unwrap(),
                        e
                    );
                    std::process::exit(1);
                }
            }
        }

        let dump_path = dump_dir.join(format!("{}.json", args.dump_name.as_ref().unwrap()));
        let dump_file = match std::fs::OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&dump_path)
        {
            Ok(f) => f,
            Err(e) => {
                log::error!("Failed to create dump file `{}`", e);
                std::process::exit(1);
            }
        };

        let w = std::io::BufWriter::new(dump_file);
        match serde_json::to_writer(w, &args) {
            Ok(_) => {
                log::info!(
                    "Dump configuration to file `{}` success!",
                    dump_path.to_str().unwrap()
                );
            }
            Err(e) => {
                log::error!(
                    "Failed to dump configuration to file `{}` (error:{})",
                    dump_path.to_str().unwrap(),
                    e
                );
                std::process::exit(1);
            }
        }
    }

    match args.benchmark.start() {
        Ok((join, cancel)) => {
            let (tx, rx) = std::sync::mpsc::sync_channel::<()>(2);
            let timer_tx = tx.clone();
            let ctrlc_tx = tx.clone();
            let _timer = std::thread::spawn(move || {
                std::thread::sleep(std::time::Duration::from_secs(args.time as u64 * 60));
                timer_tx.send(()).unwrap();
            });

            ctrlc::set_handler(move || {
                log::info!("Exiting gracefully...");
                std::thread::sleep(std::time::Duration::from_secs(1));
                ctrlc_tx.send(()).unwrap();
            })
            .map_err(|e| new_rerr!(OSERR, "Failed to set Ctrl-C handler: {}", e))
            .unwrap();

            let _cancel = std::thread::spawn(move || {
                let _ = rx.recv().unwrap();
                cancel.cancel();
            });

            if args.stat {
                start_stat(u32::max(1, args.interval));
            }

            match join.join() {
                Ok(_) => {
                    if args.stat {
                        stop_stat();
                    }
                    log::info!("Benchmark finished");
                    std::process::exit(0);
                }
                Err(e) => {
                    log::error!("{}", e);
                    std::process::exit(1);
                }
            }
        }
        Err(e) => {
            log::error!("{}", e);
            std::process::exit(1);
        }
    }
}

fn parse_page_size_str<S: AsRef<str>>(s: S) -> Result<usize> {
    let s = s.as_ref().to_ascii_lowercase();
    let page_size_str = s.trim();

    if page_size_str.ends_with("k") {
        let page_size_str = page_size_str.strip_suffix("k").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?
            * 1024)
    } else if page_size_str.ends_with("kb") {
        let page_size_str = page_size_str.strip_suffix("kb").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?
            * 1024)
    } else if page_size_str.ends_with("m") {
        let page_size_str = page_size_str.strip_suffix("m").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?
            * 1024
            * 1024)
    } else if page_size_str.ends_with("mb") {
        let page_size_str = page_size_str.strip_suffix("mb").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?
            * 1024
            * 1024)
    } else if page_size_str.ends_with("g") {
        let page_size_str = page_size_str.strip_suffix("g").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?
            * 1024
            * 1024
            * 1024)
    } else if page_size_str.ends_with("gb") {
        let page_size_str = page_size_str.strip_suffix("gb").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?
            * 1024
            * 1024
            * 1024)
    } else if page_size_str.ends_with("b") {
        let page_size_str = page_size_str.strip_suffix("b").unwrap().trim_end();
        Ok(page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?)
    } else {
        page_size_str
            .parse::<usize>()
            .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))
    }
}

fn default_page_size() -> usize {
    unsafe { libc::sysconf(libc::_SC_PAGESIZE) as usize }
}

fn supported_page_size(page_size: usize) -> bool {
    match page_size {
        // 1K
        1024 => true,
        // 4K
        4096 => true,
        // 8K
        8192 => true,
        // 16K
        16384 => true,
        // 32K
        32768 => true,
        // 64K
        65536 => true,
        // 1M
        1048576 => true,
        // 2M
        2097152 => true,
        // 1G
        1073741824 => true,
        _ => false,
    }
}
