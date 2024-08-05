use serde;
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct RAPLZone {
    #[serde(skip_serializing, skip_deserializing)]
    pub core: u32,
    pub name: String,
    pub which: u32,
    pub unit: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RAPLData {
    #[serde(skip_serializing, skip_deserializing)]
    pub zone: RAPLZone,
    pub zone_name: String,
    pub time_elapsed: u64,
    pub prev_power_read: u64,
    pub total_power_j: f64, // total consumed energy in Joules from the begining of the measurement
    pub delta_power_j: f64,
    pub watt: f64,
    pub avg_watt: f64,
    // pub temp: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct IsolateData {
    pub(crate) delta_power_j: StatData,
    pub(crate) watt: StatData,
    pub(crate) avg_watt: StatData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct StatData {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub total: f64,
}
