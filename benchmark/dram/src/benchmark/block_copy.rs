use clap::Parser;
use serde::Serialize;

use crate::MemSegment;

use super::Result;

#[derive(Debug, Parser, Serialize)]
pub struct BlockCopyBenchmark {}

impl BlockCopyBenchmark {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(&mut self) -> Result<()> {
        todo!()
    }
}

fn benchmark_start(node_id: u16, block_size: usize) -> Result<()> {
    log::info!("Starting block copy benchmark");
    let rate: u64 = 10000000000; // 10Gbit/s
    let scale = 100;
    let capacity = scale * 2048;
    let mut last_tsc = 0;
    let start_tsc = rtsc_time::Instant::now();
    const DURATION: u64 = 60 * 5;
    let src_segment = MemSegment::new(node_id as u16, 2048, 2048)?;
    let dst_segment = MemSegment::new(node_id, 2048, 2048)?;

    loop {
        let now = rtsc_time::Instant::now();
        if now.duration_since(start_tsc).as_secs() >= 1 {
            break;
        }

        break;
    }

    todo!()
}

use std::{collections::HashMap, ptr::NonNull, sync::Mutex};

use errore::{
    error::RError,
    kind::{FFIERR, IOERR},
};
use once_cell::sync::Lazy;

#[link(name = "cpuid")]
extern "C" {
    // pub fn cpuid_get_raw_data(data: *mut cpu_raw_data_t) -> ::std::os::raw::c_int;
    pub fn cpuid_get_total_cpus() -> ::std::os::raw::c_int;
    // pub fn cpuid_lib_version() -> *const ::std::os::raw::c_char;
    // pub fn cpuid_present() -> ::std::os::raw::c_int;
    // pub fn cpu_identify(raw: *mut cpu_raw_data_t, data: *mut cpu_id_t) -> ::std::os::raw::c_int;
    pub fn cpuid_error() -> *const ::std::os::raw::c_char;
    pub fn cpu_msr_driver_open_core(core_num: ::std::os::raw::c_uint) -> *mut msr_driver_t;
    pub fn cpu_rdmsr(
        handle: *mut msr_driver_t,
        msr_index: u32,
        result: *mut u64,
    ) -> ::std::os::raw::c_int;
    pub fn cpu_msr_driver_close(handle: *mut msr_driver_t) -> ::std::os::raw::c_int;
}

#[inline]
fn cpuid_error_str() -> String {
    unsafe {
        std::ffi::CStr::from_ptr(cpuid_error() as *const _)
            .to_str()
            .unwrap()
            .to_string()
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct msr_driver_t {
    _unused: [u8; 0],
}

static MSR_DRIVERS: Lazy<Mutex<HashMap<u32, MsrDriver>>> = Lazy::new(|| Mutex::new(HashMap::new()));

struct MsrDriver {
    inner: NonNull<msr_driver_t>,
}

unsafe impl Send for MsrDriver {}

impl MsrDriver {
    fn new(pkg: u32) -> Result<Self> {
        let mut core = 0;
        for c in 0..unsafe { cpuid_get_total_cpus() } {
            if cpu::topology()
                .package(pkg as u16)
                .unwrap()
                .lcores
                .is_set(c as usize)
            {
                core = c;
                break;
            }
        }

        let ptr = unsafe { cpu_msr_driver_open_core(core as std::os::raw::c_uint) };
        if ptr.is_null() {
            crate::throw_rerr!(
                FFIERR,
                "Failed to open msr driver at core {}. (error: {})",
                core,
                cpuid_error_str()
            );
        }

        Ok(Self {
            inner: NonNull::new(ptr).unwrap(),
        })
    }

    fn read(&mut self, which: u32) -> Result<u64> {
        unsafe {
            let handle = self.inner.as_mut() as *mut _;
            let mut result: u64 = 0;
            let res = cpu_rdmsr(handle, which, &mut result as *mut _);
            if res != 0 {
                crate::throw_rerr!(
                    IOERR,
                    "Failed to read msr at {}. (error:{})",
                    which,
                    cpuid_error_str()
                );
            }
            Ok(result)
        }
    }
}

impl Drop for MsrDriver {
    fn drop(&mut self) {
        unsafe {
            let ptr = self.inner.as_mut() as *mut _;
            assert!(cpu_msr_driver_close(ptr) != 0);
        }
    }
}

fn read_msr(pkg: u32, which: u32) -> Result<u64> {
    let mut msr_drivers = MSR_DRIVERS.lock().unwrap();
    if let Some(msr) = msr_drivers.get_mut(&pkg) {
        msr.read(which)
    } else {
        let mut msr = MsrDriver::new(pkg)?;
        let res = msr.read(which);
        msr_drivers.insert(pkg, msr);
        res
    }
}

fn get_total_cpu_enegy() -> Result<f64> {
    let mut total_energy = 0.0;
    let total_packages = cpu::topology().max_num_packages();
    for i in 0..total_packages {
        let unit = read_rapl_energy_unit(i as u32);
        let energy = read_msr(i as u32, 1553)?;
        total_energy += energy as f64 * unit;
    }
    Ok(total_energy)
}

fn get_total_dram_energy() -> Result<f64> {
    let mut total_energy = 0.0;
    let total_packages = cpu::topology().max_num_packages();
    for i in 0..total_packages {
        let unit = read_rapl_energy_unit(i as u32);
        let energy = read_msr(i as u32, 1561)?;
        total_energy += energy as f64 * unit;
    }
    Ok(total_energy)
}

fn read_rapl_energy_unit(pkg: u32) -> f64 {
    const MSR_RAPL_POWER_UNIT: u32 = 1542;
    const ENERGY_STATUS_UNIT_MASK: u64 = 0xF00;
    const ENERGY_STATUS_UNIT_SHIFT: usize = 8;
    let rapl_power_unit = read_msr(pkg, MSR_RAPL_POWER_UNIT).unwrap();
    let raw_energy_status_units =
        (rapl_power_unit & ENERGY_STATUS_UNIT_MASK) >> ENERGY_STATUS_UNIT_SHIFT;
    1.0 / (1 << raw_energy_status_units as usize) as f64
}
