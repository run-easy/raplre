#[cfg(not(target_os = "linux"))]
compile_error!("rust-rapl only support linux");

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[macro_use]
mod error;
pub use error::*;

mod common;
mod cpuid;
mod models;
mod tool;

#[derive(Debug, Parser)]
#[command(version = "1.0.0")]
#[command(name = "run_rapl")]
#[command(about = "Power consumption measurement tool based on Intel RAPL")]
struct Arg {
    #[arg(
        short = 'd',
        long = "delay",
        default_value_t = 1000,
        value_name = "MICROSECOND",
        help = "Delay between polls(us)"
    )]
    delay: u32,
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
            short = 'i',
            long = "interval",
            help = "Interval between benchmark runs in seconds",
            default_value_t = 0
        )]
        interval: u32,
    },

    /// Measure power consumption of an interactive application.
    ///
    /// Benchmark an interactive program. By default, `benchmark-int` expects <program> to be executable -
    /// alternatively you can specify a runner. To retain availability of the terminal and only log in the background, pass
    /// `-b, --bg-log`.
    BenchmarkInt {
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
        #[arg(short = 'b', long = "bg-log", default_value_t = false)]
        background_log: bool,
    },
    /// List utility for various RAPL-related information.
    List {
        /// What to list
        input: String,
    },
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
        #[arg(short = 'd', long = "output-dir", help = "Output file directory")]
        output_dir: Option<PathBuf>,
        /// Output file name
        output: String,
    },
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    env_logger::init();
    let arg = Arg::parse();
    match arg.tool {
        Tool::Live => {}
        _ => {}
    };
}
