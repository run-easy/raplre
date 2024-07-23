use serde;
use serde::{Deserialize, Serialize};

/// The specific RAPL domains available in a platform vary across product segments.
/// Platforms targeting the client segment support the following RAPL domain hierarchy:
/// - Package
/// - PowerPlane0 and PowerPlane1 (PowerPlane1 may reflect to uncore devices)
///
/// Platforms targeting the server segment support the following RAPL domain hierarchy:
/// - Pacakge
/// - PowerPlane0
/// - DRAM
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(crate) struct RAPLCaps(u64);

impl RAPLCaps {
    pub(crate) fn new() -> Self {
        Self(0)
    }

    pub(crate) fn has_domain(&self, domain: RAPLDomain) -> bool {
        (self.0 & domain as u64) != 0
    }

    pub(crate) fn set_domain(&mut self, domain: RAPLDomain) {
        self.0 = self.0 | domain as u64
    }

    pub(crate) fn has_msr(&self, msr: RAPLMsr) -> bool {
        (self.0 & msr as u64) != 0
    }

    pub(crate) fn set_msr(&mut self, msr: RAPLMsr) {
        self.0 = self.0 | msr as u64;
    }
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RAPLDomain {
    Pacakge = 1 << 0,
    Pp0 = 1 << 1,
    Pp1 = 1 << 2,
    DRam = 1 << 3,
}

#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RAPLMsr {
    PkgPowerLimit = 1 << 8,
    PkgEnergyStatus = 1 << 9,
    PkgPowerInfo = 1 << 10,
    PkgPerfStatus = 1 << 11,
    Pp0PowerLimit = 1 << 16,
    Pp0EnergyStatus = 1 << 17,
    Pp0Policy = 1 << 18,
    Pp0PerfStatus = 1 << 19,
    Pp1PowerLimit = 1 << 24,
    Pp1EnergyStatus = 1 << 25,
    Pp1Policy = 1 << 26,
    Pp1PerfStatus = 1 << 27,
    DramPowerLimit = 1 << 32,
    DramPowerInfo = 1 << 33,
    DramPerfStatus = 1 << 34,
    DramEnergyStatus = 1 << 35,
    PlatformEnergyStatus = 1 << 36,
    PlatformPowerLimit = 1 << 37,
    PlatformPowerInfo = 1 << 38,
    PlatformRaplSocketPerfStatus = 1 << 39,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RAPLData {
    pub package: u32,
    pub caps: RAPLCaps,
    pub time_elapsed: f64,
    pub power_j: f64,
    pub watts: f64,
    pub watts_since_last: f64,
    pub prev_power: f64,
    pub prev_power_reading: f64,
    pub temp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IsolateData {
    pub power_j: StatData,
    pub watts: StatData,
    pub watts_since_last: StatData,
    pub watt_h: StatData,
    pub kwatt_h: StatData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StatData {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub total: f64,
}
