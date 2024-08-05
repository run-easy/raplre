mod simple;
pub use simple::*;
mod block_copy;
pub use block_copy::*;

use crate::*;

pub struct BenchmarkJoinHandle {
    join_handle: Box<dyn FnOnce() -> Result<()>>,
}

unsafe impl Send for BenchmarkJoinHandle {}

impl BenchmarkJoinHandle {
    pub fn join(self) -> Result<()> {
        let this = self;
        (this.join_handle)()
    }
}

unsafe impl Send for BenchmarkCancelHandle {}
pub struct BenchmarkCancelHandle {
    cancel_handle: Box<dyn FnOnce()>,
}

impl BenchmarkCancelHandle {
    pub fn cancel(self) {
        let this = self;
        (this.cancel_handle)()
    }
}

pub enum PktSizeDistr {
    Constant(u16),
    // Uniform(u16, u16),
}

impl PktSizeDistr {
    fn parse<S: AsRef<str>>(s: S) -> Result<Self> {
        let s = s.as_ref();
        let s = s.trim();
        if !s.ends_with(")") {
            throw_rerr!(
                INVALID_VALUE,
                "Failed to parse packet size distribution,invalid format `{}`",
                s
            );
        }
        let s = s.strip_suffix(")").unwrap();

        let distr_type = match s.split('(').nth(0) {
            Some(d) => d,
            None => {
                throw_rerr!(
                    INVALID_VALUE,
                    "Failed to parse packet size distribution,invalid format `{}`",
                    s
                );
            }
        }
        .to_ascii_lowercase();

        let params = match s.split('(').nth(1) {
            Some(p) => p,
            None => throw_rerr!(
                INVALID_VALUE,
                "Failed to parse packet size distribution,invalid format `{}`",
                s
            ),
        }
        .split(",")
        .collect::<Vec<&str>>()
        .iter_mut()
        .map(|params| {
            *params = params.trim();
            if params.ends_with(")") {
                *params = params.strip_suffix(")").unwrap();
            }
            *params
        })
        .collect::<Vec<&str>>();

        match distr_type.as_str() {
            "constant" => {
                if params.len() == 0 {
                    throw_rerr!(
                        INVALID_VALUE,
                        "Failed to parse packet size distribution,constant need a parameter, like this `constant(64)`",
                    );
                }

                if params.len() != 1 {
                    throw_rerr!(
                        INVALID_VALUE,
                        "Failed to parse packet size distribution,too many parameters,only one is need but {} are given",
                        params.len()
                    );
                }

                let size = params[0].parse::<u16>().map_err(|e| {
                    new_rerr!(
                        INVALID_VALUE,
                        "Failed to parse packet size distribution,invalid parameter (error:{})",
                        e
                    )
                })?;
                Ok(Self::Constant(size))
            }
            // "uniform" => {
            //     if params.len() != 2 {
            //         throw_rerr!(INVALID_VALUE, "Invalid format `{}`", s);
            //     }
            //     let min = params[0]
            //         .parse::<u16>()
            //         .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?;
            //     let max = params[1]
            //         .parse::<u16>()
            //         .map_err(|e| new_rerr!(INVALID_VALUE, "{}", e))?;
            //     Ok(Self::Uniform(min, max))
            // }
            _ => throw_rerr!(
                INVALID_VALUE,
                "Failed to parse packet size distribution,unknown packet size distribution `{}`",
                distr_type
            ),
        }
    }

    pub fn generate(&self) -> Box<dyn FnMut() -> u16> {
        match self {
            Self::Constant(size) => {
                let size = *size;
                Box::new(move || size)
            } // Self::Uniform(min, max) => {
              //     let mut rng = rand::thread_rng();
              //     Box::new(move || rng.gen_range(min..=max))
              // }
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, ValueEnum, Serialize)]
pub enum MoveMentMode {
    R2R,
    R2S,
    S2R,
    S2S,
}
