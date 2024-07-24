use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Instant, SystemTime},
};

use crate::{
    cpuid::get_core_of_package,
    models::{self, IsolateData, RAPLZone},
};

// measurement thread
pub(crate) const THREAD_KILL: i8 = 1;

// ncurses
const COLOUR_BLACK: i16 = 0;
const DEFAULT_COLOUR: i16 = -1;
pub(crate) const HEADER_PAIR: i16 = 1;
pub(crate) const KEY_CODE_EXIT: i32 = 113; // q

pub(crate) fn setup_ncurses() {
    let w = ncurses::initscr();
    ncurses::curs_set(ncurses::CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    ncurses::nodelay(w, true);

    if ncurses::has_colors() {
        ncurses::start_color();
        ncurses::init_pair(HEADER_PAIR, COLOUR_BLACK, DEFAULT_COLOUR);
    }
}

pub(crate) fn kill_ncurses() {
    ncurses::endwin();
    ncurses::reset_shell_mode();
}

pub(crate) fn should_terminate(limit: u64, now: Instant, start_time: Instant) -> bool {
    return limit > 0 && now.duration_since(start_time).as_secs() >= limit;
}

pub(crate) fn terminate() {
    kill_ncurses();
}

pub(crate) fn setup_rapl_data() -> Vec<models::RAPLData> {
    detect_cpu()
        .into_iter()
        .map(|zone| models::RAPLData {
            zone_name: zone.name.clone(),
            time_elapsed: 0,
            zone: zone,
            prev_power_read: 0,
            total_power_j: 0.0,
            watt: 0.0,
            avg_watt: 0.0,
            delta_power_j: 0.0,
        })
        .collect()
}

pub(crate) fn detect_cpu() -> Vec<RAPLZone> {
    let mut zones = vec![];
    for pkg in 0..crate::cpuid::num_package() {
        let cpu_info = crate::cpuid::get_cpu_info(pkg).unwrap();
        if cpu_info.vendor_name != "GenuineIntel" {
            eprintln!(
                "Detected `{}`({}) cpu, run-rapl only support Intel CPU.",
                cpu_info.vendor_name, cpu_info.vendor,
            );
            std::process::exit(1);
        }

        let core = get_core_of_package(pkg).unwrap();

        // Sapphire Rapids microarchitecture. It supports the follows MSRs:
        if cpu_info.display_family == 0x06 && cpu_info.display_model == 0x8f {
            // MSR_PP0_ENERGY_STATUS
            // MSR_PP0_ENERGY_STATUS/MSR_PP1_ENERGY_STATUS are read-only MSRs. They report the actual energy use
            // for the respective power plane domains. These MSRs are updated every ~1msec.
            //
            // Total Energy Consumed (31:0): The unsigned integer value represents the total amount of energy consumed since the last time this register was cleared.
            // The unit of this field is specified by the “Energy Status Units” field of MSR_RAPL_POWER_UNIT.
            zones.push(RAPLZone {
                core: core,
                name: format!("Pp0-{}", pkg),
                which: 1593,
                unit: read_rapl_energy_unit(pkg),
            });

            // MSR_DRAM_ENERGY_STATUS

            // Energy in 61 micro-joules. Requires BIOS configuration to enable DRAM
            // RAPL mode 0 (Direct VR).
            zones.push(RAPLZone {
                core: core,
                name: format!("Dram-{}", pkg),
                which: 1561,
                unit: 0.000061,
            });

            // MSR_PKG_ENERGY_STATUS
            //
            // Total energy consumption in J (32.0), in 10nsec units.
            zones.push(RAPLZone {
                core: core,
                name: format!("Package-{}", pkg),
                which: 1553,
                unit: read_rapl_energy_unit(pkg),
            });
        }
    }

    zones
}

pub(crate) fn update_measurements(
    zones: &mut Vec<models::RAPLData>,
    now: Instant,
    start_time: Instant,
    prev_time: Instant,
    output_file: &PathBuf,
    isolate_map: Option<&HashMap<String, models::IsolateData>>,
) {
    for zone in zones.iter_mut() {
        match isolate_map {
            Some(map) => {
                let iz = map.get(&zone.zone.name).unwrap();
                calculate_isolated_power_metrics(zone, now, start_time, prev_time, iz)
            }
            _ => calculate_power_metrics(zone, now, start_time, prev_time),
        }

        crate::logger::log_poll_result(zone, output_file);
    }
}

fn read_rapl_energy_unit(pkg: u32) -> f64 {
    const MSR_RAPL_POWER_UNIT: u32 = 1542;
    const ENERGY_STATUS_UNIT_MASK: u64 = 0xF00;
    const ENERGY_STATUS_UNIT_SHIFT: usize = 8;
    let core = get_core_of_package(pkg).unwrap();
    let rapl_power_unit = crate::cpuid::read_msr(core, MSR_RAPL_POWER_UNIT).unwrap();
    let raw_energy_status_units =
        (rapl_power_unit & ENERGY_STATUS_UNIT_MASK) >> ENERGY_STATUS_UNIT_SHIFT;
    1.0 / (1 << raw_energy_status_units as usize) as f64
}

pub(crate) fn calculate_isolated_power_metrics(
    zone: &mut models::RAPLData,
    now: Instant,
    start_time: Instant,
    prev_time: Instant,
    isolated_zone: &models::IsolateData,
) {
    calculate_power_metrics(zone, now, start_time, prev_time);
    zone.total_power_j -= zone.delta_power_j;
    zone.delta_power_j -= isolated_zone.delta_power_j.avg;
    zone.total_power_j += zone.delta_power_j;
    zone.watt -= isolated_zone.watt.avg;
    zone.avg_watt -= isolated_zone.avg_watt.avg;
}

fn calculate_power_metrics(
    zone: &mut models::RAPLData,
    now: Instant,
    start_time: Instant,
    prev_time: Instant,
) {
    let cur_power = crate::cpuid::read_msr(zone.zone.core, zone.zone.which).unwrap();

    #[allow(unused_assignments)]
    let mut delta_power = 0;

    // if RAPL overflow has occurred
    //
    // Assuming the energy unit is 1 micro-joule, the maximum value for each 32-bit
    // sensor register is 4,294,967,295, which defines the boundary for upward overflow.
    // According to Intel's user manual, these registers are updated more than 100 times
    // per second. Therefore, when we read the register value every 10 milliseconds,
    // a RAPL (Running Average Power Limit) cycle will never complete within a read interval,
    // as doing so would require the computer to operate at a power level of 429,496 watts.
    if zone.prev_power_read > cur_power {
        delta_power = (u32::MAX as u64 - zone.prev_power_read) + cur_power;
    } else {
        delta_power = cur_power - zone.prev_power_read;
    }

    // Reset the delta_power consumption to zero during the initial measurement.
    // This ensures that the total energy consumption for each RAPL domain begins from zero.
    if prev_time == start_time {
        delta_power = 0;
    }

    let delta_power_j = delta_power as f64 * zone.zone.unit;

    zone.total_power_j += delta_power_j;
    zone.delta_power_j = delta_power_j;
    zone.prev_power_read = cur_power;

    // Calculate the instantaneous power in Watt(J/s)
    zone.watt = delta_power_j * 1000.0 / (now.duration_since(prev_time).as_millis() as f64);
    // Calculate the average power in Watt(J/s) over the time from the beginning of the measurement to the present.
    zone.avg_watt =
        zone.total_power_j * 1000.0 / (now.duration_since(start_time).as_millis() as f64);

    zone.time_elapsed = now.duration_since(start_time).as_secs();
}

pub(crate) fn read_isolated_data(
    isolate_file: Option<&PathBuf>,
) -> Option<HashMap<String, IsolateData>> {
    isolate_file.map(|file_path| {
        let data = std::fs::read(file_path).expect("Couldn't read file");
        let map: HashMap<String, models::IsolateData> =
            serde_json::from_str(String::from_utf8(data).unwrap().as_str()).unwrap();
        map
    })
}

pub(crate) fn get_last_measurement_from(file: PathBuf) -> Vec<models::RAPLData> {
    let mut rdr = csv::Reader::from_path(file).unwrap();
    let zones = detect_cpu();
    let mut out: Vec<models::RAPLData> = vec![];
    for res in rdr.deserialize() {
        let r: models::RAPLData = res.unwrap();
        out.push(r);
    }

    let last = &out[out.len() - zones.len()..].to_vec();

    return last.to_owned();
}

pub(crate) fn create_log_file_name<S: AsRef<str>, T: AsRef<str>>(
    // dir: Option<&PathBuf>,
    benchmark_name: S,
    tool: T,
    system_start_time: SystemTime,
) -> String {
    let time = system_start_time
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Failed to check duration")
        .as_secs();
    return format!("{}-{}-{}.csv", benchmark_name.as_ref(), tool.as_ref(), time);
}

pub(crate) fn spacing<S: AsRef<str>>(line: S) -> String {
    // 25 and 30 makes for fucky formatting:
    // w/h is misaligned; line break on small monitors, respectively
    let col_spacing = 28;
    return " ".repeat(col_spacing - line.as_ref().len());
}

pub(crate) fn print_headers(ncurses: bool) {
    let headers = vec![
        "zone", "time(s)", "J", // total energy consumed
        "delta_J", "avg J/s", "J/s",
    ];
    let mut line: String = "".to_owned();

    for h in headers {
        line.push_str(format!("{}{}", h, spacing(h.to_string())).as_str());
    }

    line = line.trim().to_string();
    line.push_str("\n");
    if ncurses {
        ncurses::attron(ncurses::A_BOLD());
        ncurses::addstr(line.as_str());
        ncurses::attroff(ncurses::A_BOLD());
        ncurses::refresh();
    } else {
        print!("{}", line);
    }
}

#[macro_export]
macro_rules! print_headers {
    ($ncurses: expr) => {
        // what the fuck
        {
            crate::common::print_headers($ncurses);
        }
    };
    () => {
        // what the fuck
        {
            crate::common::print_headers(false);
        }
    };
}

pub(crate) fn print_result_line(zones: &Vec<models::RAPLData>, ncurses: bool) {
    let mut line: String = "\r".repeat(zones.len()).to_owned();

    for zone in zones {
        let fields = vec![
            zone.time_elapsed as f64,
            zone.total_power_j,
            zone.delta_power_j,
            zone.avg_watt,
            zone.watt,
        ];
        let zone_name = zone.zone_name.as_str();
        line.push_str(format!("{}{}", zone_name, spacing(zone_name)).as_str());

        for f in fields {
            line.push_str(format!("{:.5}{}", f, spacing(format!("{:.5}", f))).as_str());
        }

        line = line.trim().to_string();

        line.push_str("\n");
    }

    if ncurses {
        print_headers!(true);
        ncurses::addstr(line.as_str());
        ncurses::refresh();
    } else {
        print!("{}", line);
    }
}

#[macro_export]
macro_rules! ncprint {
    ($str: expr) => {
        ncurses::addstr($str);
        ncurses::refresh();
    };
}

#[macro_export]
macro_rules! print_result_line {
    ($zones: expr, $ncurses: expr) => {
        // what the fuck
        {
            crate::common::print_result_line($zones, $ncurses);
        }
    };
    ($zones: expr) => {
        // what the fuck
        {
            crate::common::print_result_line($zones, false);
        }
    };
}
