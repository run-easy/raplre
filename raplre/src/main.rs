#[cfg(not(target_os = "linux"))]
compile_error!("rust-rapl only support linux");

use clap::{Parser, Subcommand};
use std::{os::unix::fs::PermissionsExt, path::PathBuf};

#[macro_use]
mod error;
pub use error::*;

#[macro_use]
mod common;
mod cpuid;
mod logger;
mod models;
mod task;
mod tool;

#[derive(Debug, Parser)]
#[command(version = "1.0.0")]
#[command(name = "run_rapl")]
#[command(about = "Power consumption measurement tool based on Intel RAPL")]
struct Arg {
    // #[arg(
    //     short = 'd',
    //     long = "delay",
    //     default_value_t = 1000,
    //     value_name = "MICROSECOND",
    //     help = "Delay between polls (ms)"
    // )]
    // delay: u32,
    #[arg(
        short = 't',
        long = "terminate-after",
        value_name = "SECOND",
        help = "Terminate after time limit(s)"
    )]
    run_time_limit: Option<u32>,
    #[arg(
        short = 'n',
        long = "name",
        value_name = "NAME",
        help = "Benchmark name - to easily discern csv output"
    )]
    name: Option<String>,
    #[arg(
        short = 'i',
        long = "isolate-from",
        value_name = "PATH",
        help = "Idel data to isolate measurement from. run-rapl can estimate 
pure software energy consumption by offsetting the measurements
using previously measurement idle data of the system consumption."
    )]
    isolate_file: Option<PathBuf>,
    #[arg(long = "dir", help = "Output directory")]
    output_dir: Option<PathBuf>,
    #[command(subcommand)]
    tool: Tool,
}

#[derive(Debug, Subcommand)]
enum Tool {
    /// Live measurement
    Live,
    /// Measure power consumption of a oneshot script.
    ///
    /// Benchmark a single, oneshot program, optionally `n` times. If -n is passed, n .csv files
    /// will be generated as well. By default, benchmark expects <program> to be executable -
    /// alternatively you can specify a runner, e.g., `bash`, with `-r, --runner`.
    /// Additionally, `benchmark` expects <program> to terminate on its own - if this is not the
    /// case for your benchmark, use benchmark-int instead.
    Benchmark {
        #[arg(
            short = 'r',
            long = "runner",
            help = "Benchmark requires <runner> to execute"
        )]
        runner: Option<PathBuf>,
        /// Benchmark program
        program: PathBuf,
        /// Args for <program>
        args: Vec<String>,
        #[arg(
            short = 'n',
            long = "count",
            default_value_t = 1,
            help = "Amount of times to run benchmark"
        )]
        count: u32,
        #[arg(
            long = "smooth",
            help = "Smoothing data based on EWMA",
            default_value_t = false
        )]
        smooth: bool,
        #[arg(
            short = 'i',
            long = "interval",
            help = "Interval between benchmark runs in seconds",
            default_value_t = 0
        )]
        interval: u32,
    },
    /// List supported RAPL domain.
    List,
    /// Pretty print last measurement of .csv file
    PrettyPrint {
        /// File to print from
        file: PathBuf,
    },
    /// Tools for measuring and generating isolation data
    Isolate {
        #[arg(
            short = 'm',
            long = "measure",
            default_value_t = 30,
            value_name = "MINUTES",
            help = "Measure data as a basis for isolation for n minutes - make sure your system is as idle as possible"
        )]
        measure: u32,
        #[arg(
            short = 'f',
            long = "from",
            help = "Generate isolation data from .csv file"
        )]
        file: Option<PathBuf>,
    },
    /// Tools for extract data from csv file.
    Extract {
        #[arg(
            long = "smooth",
            default_value_t = false,
            help = "Smoothing data based on EWMA"
        )]
        smooth: bool,
        #[arg(
            long = "alpha",
            default_value_t = 0.02,
            help = "A constant double float value between 0 and 1, used for smoothing data based on EWMA"
        )]
        alpha: f64,
        /// File to extract
        file: PathBuf,
    },
}

// Disk Write: < 5 KB/s
const POLL_DELAY: u32 = 20;

fn main() {
    let arg = Arg::parse();

    if arg.output_dir.is_some() {
        let path = std::path::Path::new(arg.output_dir.as_ref().unwrap());
        if !path.exists() {
            std::fs::create_dir_all(path).unwrap();
            let mut perm = std::fs::metadata(path).unwrap().permissions();
            perm.set_mode(0o775);
            std::fs::set_permissions(path, perm).unwrap();
        }
    }

    if let Err(e) = match arg.tool {
        Tool::Live => {
            crate::common::setup_ncurses();
            crate::tool::live_measurement(
                POLL_DELAY,
                arg.run_time_limit,
                arg.name.as_ref(),
                arg.output_dir.as_ref(),
            )
        }
        Tool::Benchmark {
            runner,
            program,
            args,
            count,
            smooth,
            interval,
        } => tool::do_benchmarks(
            POLL_DELAY,
            arg.name.as_ref(),
            arg.output_dir.as_ref(),
            arg.isolate_file.as_ref(),
            runner,
            program,
            args,
            count,
            smooth,
            interval,
        ),
        Tool::List => tool::list(),
        Tool::PrettyPrint { file } => tool::pretty_print(file),
        Tool::Isolate { measure, file } => {
            match file {
                Some(path) => {
                    // generate data
                    tool::generate_isolate_data(
                        arg.output_dir.as_ref(),
                        arg.name.as_ref().unwrap_or(&"default".to_string()),
                        path,
                    )
                }
                _ => {
                    common::setup_ncurses();
                    // measure data basis
                    tool::measure_isolate_data(
                        POLL_DELAY,
                        arg.output_dir.as_ref(),
                        arg.name.as_ref().unwrap_or(&"default".to_string()),
                        measure,
                    )
                }
            }
        }
        Tool::Extract {
            smooth,
            alpha,
            file,
        } => tool::extract_data(
            arg.output_dir.as_ref(),
            arg.name.as_ref().unwrap_or(&"default".to_string()),
            smooth,
            alpha,
            file,
        ), // _ => unreachable!(),
    } {
        eprintln!("ERROR:{}", e);
        std::process::exit(1);
    }

    std::process::exit(0);
}
