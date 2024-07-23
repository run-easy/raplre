use std::collections::HashSet;

use clap::Parser;
use libc::size_t;

#[derive(Debug, Parser)]
struct Arg {
    #[arg(short = 'l', help = "Logical cores that loopers start to run on")]
    lcore: String,
    #[arg(
        short = 'p',
        help = "Use pause instruction instead.",
        default_value_t = false
    )]
    pause: bool,
    #[arg(short = 'd', help = "Duration time in seconds")]
    duration: Option<u32>,
}

impl Arg {
    pub fn parse_cores(&self) -> Vec<u32> {
        let zones: Vec<&str> = self.lcore.split(",").collect();
        let mut cores = HashSet::new();
        for zone in zones {
            if zone.contains("-") {
                let splits: Vec<&str> = zone.split("-").collect();
                assert_eq!(splits.len(), 2);
                let start: u32 = splits[0].parse().unwrap();
                let end: u32 = splits[1].parse().unwrap();

                for core in start..=end {
                    cores.insert(core);
                }
            } else {
                cores.insert(zone.parse::<u32>().unwrap());
            }
        }

        cores.into_iter().collect()
    }
}

fn set_cpu_affinity(core: u32) {
    unsafe {
        let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut cpuset);
        libc::CPU_SET(core as usize, &mut cpuset);
        libc::pthread_setaffinity_np(
            libc::pthread_self(),
            libc::CPU_SETSIZE as size_t,
            &mut cpuset as *mut _,
        );
    }
}

fn main() {
    let arg = Arg::parse();

    let cores = arg.parse_cores();

    for core in cores {
        if arg.pause {
            std::thread::spawn(move || {
                set_cpu_affinity(core);
                loop {
                    unsafe {
                        core::arch::x86_64::_mm_pause();
                    }
                }
            });
        } else {
            std::thread::spawn(move || {
                set_cpu_affinity(core);
                loop {}
            });
        };
    }

    std::thread::sleep(std::time::Duration::from_secs(
        arg.duration.unwrap_or(u32::MAX) as u64,
    ));
}
