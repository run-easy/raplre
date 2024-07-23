use std::{collections::HashMap, io::Read};

use libc::pthread_setaffinity_np;
use once_cell::sync::OnceCell;
use run_error::{
    error::RError,
    kind::{RErrorKind, FFIERR, OSERR},
};

use crate::models;

const MAX_NUM_CORES: usize = 1024;
const MAX_NUM_PACKAGES: usize = 16;

static PKG_MAP: OnceCell<HashMap<u32, u32>> = OnceCell::new();

fn pkg_map_init() -> HashMap<u32, u32> {
    let mut packages = HashMap::new();
    for core in 0..MAX_NUM_CORES {
        let path = format!(
            "/sys/bus/cpu/devices/cpu{}/topology/physical_package_id",
            core
        );
        if let Ok(mut f) = std::fs::File::open(&path) {
            let mut pkg_id_str = String::new();
            if let Err(e) = f.read_to_string(&mut pkg_id_str) {
                log::error!("Failed to read {path}: {e}.");
                std::process::exit(1);
            }
            let pkg_id: u32 = pkg_id_str.trim().parse().unwrap();
            packages.insert(pkg_id, core as u32);
        } else {
            log::error!("Failed to open {path}.");
            std::process::exit(1);
        }
    }
    packages
}

pub(crate) fn num_package() -> u32 {
    let pkg_map = PKG_MAP.get_or_init(|| pkg_map_init());
    pkg_map.len() as u32
}

/// Model specific registers and its bit-fields may be supported for a finite range of processor families/models.
/// To distinguish between different processor family and/or models, software must use CPUID.01H leaf function to query the
/// combination of DisplayFamily and DisplayModel to determine model-specific availability of MSRs.
///
/// references: 2024 Intel 64 and IA-32 Architectures Software Developer's Manual
pub(crate) fn list_rapl(package: u32) -> Result<models::RAPLCaps, RError> {
    let pkg_map = PKG_MAP.get_or_init(|| pkg_map_init());
    if !pkg_map.contains_key(&package) {
        crate::throw_rerr!(crate::INVALID_VALUE, "Invalid package id {}", package);
    }
    let core = *pkg_map.get(&package).unwrap();
    let mut cpu_data = crate::cpuid::cpuid_get_raw_data_core(core)?;
    let (family, model) = unsafe {
        let mut cpu_info: crate::cpuid::cpu_id_t = unsafe { std::mem::zeroed() };
        let res = crate::cpuid::cpu_identify(&mut cpu_data as *mut _, &mut cpu_info as *mut _);
        if res != 0 {
            crate::throw_rerr!(
                FFIERR,
                "Failed to identify cpu {}. (error: {})",
                core,
                std::ffi::CStr::from_ptr(crate::cpuid::cpuid_error())
                    .to_str()
                    .unwrap()
            );
        }

        if cpu_info.vendor != 0x806EC {
            crate::throw_rerr!(
                RErrorKind::new_custom(100),
                "Detected non-intel CPU({}),run-rapl only support Intel CPU.",
                std::ffi::CStr::from_ptr(cpu_info.vendor_str.as_ptr())
                    .to_str()
                    .unwrap()
            );
        }

        (cpu_info.family as u32, cpu_info.model as u32)
    };

    let mut caps = models::RAPLCaps::new();
    let id = family << 8 | model;

    // // Silvermont
    // if (id == 0x06_37 || id == 0x06_4A || id == 0x06_5A || id == 0x06_5D) {
    //     caps.set_domain(models::RAPLDomain::Pacakge);
    //     caps.set_domain(models::RAPLDomain::Pp0);
    //     caps.set_msr(models::RAPLMsr::PkgPowerLimit);
    //     caps.set_msr(models::RAPLMsr::PkgEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    // }

    // if (id == 0x06_4d) {
    //     caps.set_domain(models::RAPLDomain::Pacakge);
    //     caps.set_msr(models::RAPLMsr::PkgPowerLimit);
    //     caps.set_msr(models::RAPLMsr::PkgPowerInfo);
    // }

    // // Airmont
    // if (id == 0x06_4C) {
    //     caps.set_domain(models::RAPLDomain::Pacakge);
    //     caps.set_domain(models::RAPLDomain::Pp0);
    //     caps.set_msr(models::RAPLMsr::PkgPowerLimit);
    //     caps.set_msr(models::RAPLMsr::PkgEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp0PowerLimit);
    // }

    // // Intel Atom® Processors Based on Goldmont,Goldmont Plus or Tremont
    // if (id == 0x06_5C || id == 0x06_7A) {
    //     caps.set_domain(models::RAPLDomain::Pacakge);
    //     caps.set_msr(models::RAPLMsr::PkgPowerLimit);
    //     caps.set_msr(models::RAPLMsr::PkgEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::PkgPerfStatus);
    //     caps.set_msr(models::RAPLMsr::PkgPowerInfo);
    //     caps.set_domain(models::RAPLDomain::DRam);
    //     caps.set_msr(models::RAPLMsr::DramEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::DramPerfStatus);
    //     caps.set_msr(models::RAPLMsr::DramPowerInfo);
    //     caps.set_msr(models::RAPLMsr::DramPowerLimit);
    //     caps.set_domain(models::RAPLDomain::Pp0);
    //     caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    //     caps.set_domain(models::RAPLDomain::Pp1);
    //     caps.set_msr(models::RAPLMsr::Pp1EnergyStatus);
    // }

    // // Sandy Bridge 2-20
    // if (id == 0x06_2A || id == 0x06_2D) {
    //     caps.set_domain(models::RAPLDomain::Pacakge);
    //     caps.set_msr(models::RAPLMsr::PkgPowerLimit);
    //     caps.set_msr(models::RAPLMsr::PkgEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::PkgPowerInfo);
    //     caps.set_domain(models::RAPLDomain::Pp0);
    //     caps.set_msr(models::RAPLMsr::Pp0PowerLimit);
    // }
    // // 2-21
    // if (id == 0x06_2A) {
    //     caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp0Policy);
    //     caps.set_domain(models::RAPLDomain::Pp1);
    //     caps.set_msr(models::RAPLMsr::Pp1PowerLimit);
    //     caps.set_msr(models::RAPLMsr::Pp1EnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp1Policy);
    // }
    // // 2-23
    // if (id == 0x06_2D) {
    //     caps.set_msr(models::RAPLMsr::PkgPerfStatus);
    //     caps.set_domain(models::RAPLDomain::DRam);
    //     caps.set_msr(models::RAPLMsr::DramEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::DramPerfStatus);
    //     caps.set_msr(models::RAPLMsr::DramPowerInfo);
    //     caps.set_msr(models::RAPLMsr::DramPowerLimit);
    //     caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    // }

    // // Ivy Bridge
    // if (id == 0x06_3A) {
    //     caps.set_domain(models::RAPLDomain::Pacakge);
    //     caps.set_msr(models::RAPLMsr::PkgPowerLimit);
    //     caps.set_msr(models::RAPLMsr::PkgEnergyStatus);
    //     caps.set_msr(models::RAPLMsr::PkgPowerInfo);
    //     caps.set_domain(models::RAPLDomain::Pp0);
    //     caps.set_msr(models::RAPLMsr::Pp0PowerLimit);
    //     caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp0Policy);
    //     caps.set_domain(models::RAPLDomain::Pp1);
    //     caps.set_msr(models::RAPLMsr::Pp1PowerLimit);
    //     caps.set_msr(models::RAPLMsr::Pp1EnergyStatus);
    //     caps.set_msr(models::RAPLMsr::Pp1Policy);
    // }

    // Sapphire Rapids microarchitecture
    if (id == 0x06_8F) {
        caps.set_domain(models::RAPLDomain::DRam);
        caps.set_msr(models::RAPLMsr::DramEnergyStatus);
        caps.set_msr(models::RAPLMsr::DramPerfStatus);
        caps.set_msr(models::RAPLMsr::DramPowerInfo);
        caps.set_msr(models::RAPLMsr::DramPowerLimit);
        caps.set_msr(models::RAPLMsr::Pp0EnergyStatus);
    }

    todo!()
}

pub(crate) fn setup_rapl_data() -> Vec<models::RAPLData> {
    let mut res = vec![];
    for pkg in 0..num_package() {
        res.push(models::RAPLData {
            package: pkg,
            caps: list_rapl(pkg).unwrap(),
            time_elapsed: 0.0,
            power_j: 0.0,
            watts: 0.0,
            watts_since_last: 0.0,
            prev_power: 0.0,
            prev_power_reading: 0.0,
            temp: 0.0,
        })
    }
    res
}
