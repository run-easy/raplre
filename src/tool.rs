use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use std::u32;

use crate::common;
use crate::error::*;
use crate::logger;
use crate::models;
use crate::models::RAPLData;
use crate::models::StatData;
use crate::task;

pub(crate) fn live_measurement(
    poll_delay: u32,
    run_time_limit: Option<u32>,
    name: Option<&String>,
    dir: Option<&PathBuf>,
) -> Result<(), RError> {
    let sleep = std::time::Duration::from_millis(poll_delay as u64);
    let mut zones = common::setup_rapl_data();
    assert!(zones.len() != 0);
    let system_start_time = SystemTime::now();
    let start_time = Instant::now();
    let run_time_limit = run_time_limit.unwrap_or(0) as u64;
    let mut prev_time = start_time;
    #[allow(unused_assignments)]
    let mut now = start_time;
    const TOOL_NAME: &'static str = "live";

    let output_file = match dir {
        Some(dir) => dir.join(common::create_log_file_name(
            name.unwrap_or(&"default".to_string()),
            TOOL_NAME,
            system_start_time,
        )),
        None => PathBuf::from(common::create_log_file_name(
            name.unwrap_or(&"default".to_string()),
            TOOL_NAME,
            system_start_time,
        )),
    };

    loop {
        now = Instant::now();
        common::update_measurements(&mut zones, now, start_time, prev_time, &output_file, None);
        prev_time = now;

        ncurses::clear();
        ncprint!("Press 'q' to quit\n");
        print_result_line!(&zones, true);

        if ncurses::getch() == common::KEY_CODE_EXIT {
            ncurses::endwin();
            break;
        }

        if common::should_terminate(run_time_limit, now, start_time) {
            common::terminate();
            break;
        }

        std::thread::sleep(sleep);

        // std::thread::sleep(std::time::Duration::from_secs(100000));
    }

    print_headers!();
    print_result_line!(&zones);
    println!();

    Ok(())
}

pub(crate) fn do_benchmarks(
    poll_delay: u32,
    name: Option<&String>,
    dir: Option<&PathBuf>,
    isolate: Option<&PathBuf>,
    runner: Option<PathBuf>,
    program: PathBuf,
    args: Vec<String>,
    count: u32,
    interval: u32,
) -> Result<(), RError> {
    let sleep = Duration::from_secs(interval as u64);
    for i in 0..count {
        if count > 1 {
            println!("Running benchmark iteration {}", i + 1);
        }

        benchmark(
            poll_delay,
            name.clone(),
            dir.clone(),
            isolate.clone(),
            runner.as_ref(),
            &program,
            &args,
        )?;

        if interval > 0 && i + 1 < count {
            println!("Sleep for {} seconds before next benchmark run", interval);
            thread::sleep(sleep);
        }
    }

    Ok(())
}

fn benchmark(
    poll_delay: u32,
    name: Option<&String>,
    dir: Option<&PathBuf>,
    isolate_file: Option<&PathBuf>,
    runner: Option<&PathBuf>,
    program: &PathBuf,
    args: &Vec<String>,
) -> Result<(), RError> {
    let isolate_map = common::read_isolated_data(isolate_file);
    let start_time = Instant::now();
    let iteration_start_time = SystemTime::now();
    const TOOL_NAME: &'static str = "benchmark";

    let output_file = match dir {
        Some(dir) => dir.join(common::create_log_file_name(
            name.unwrap_or(&"default".to_string()),
            TOOL_NAME,
            iteration_start_time,
        )),
        None => PathBuf::from(common::create_log_file_name(
            name.unwrap_or(&"default".to_string()),
            TOOL_NAME,
            iteration_start_time,
        )),
    };

    let (send, recv) = mpsc::channel();

    let thr = task::spawn_measurement_thread(
        start_time,
        recv,
        poll_delay,
        output_file.clone(),
        isolate_map,
    );

    match runner {
        Some(r) => {
            let _out = Command::new(r)
                .arg(program)
                .args(args)
                .stderr(Stdio::null())
                .stdout(Stdio::null())
                .output()
                .expect("Failed to execute command");
        }
        None => {
            let _out = Command::new(program)
                .args(args)
                .stderr(Stdio::null())
                .stdout(Stdio::null())
                .output()
                .expect("Failed to execute command");
        }
    }

    send.send(common::THREAD_KILL)
        .expect("Failed to concact measurement thread");
    thr.join()
        .expect("Failed to wait for measurement thread to finish");

    let new_zones = common::get_last_measurement_from(output_file);

    print_headers!(false);
    print_result_line!(&new_zones);
    println!();

    Ok(())
}

pub(crate) fn list() -> Result<(), RError> {
    let zones = common::detect_cpu();
    for zone in zones {
        println!("{}", zone.name);
    }

    Ok(())
}

pub(crate) fn pretty_print(file: PathBuf) -> Result<(), RError> {
    if !file.exists() {
        crate::throw_rerr!(NOT_FOUND, "Failed to find `{}`", file.to_str().unwrap());
    }

    if !file.is_file() {
        crate::throw_rerr!(
            NOT_ALLOWED,
            "Path `{}` is not a file",
            file.to_str().unwrap()
        );
    }

    let last = common::get_last_measurement_from(file);

    print_headers!();
    print_result_line!(&last);
    println!();

    Ok(())
}

pub(crate) fn generate_isolate_data(
    dir: Option<&PathBuf>,
    name: &String,
    csv_file: PathBuf,
) -> Result<(), RError> {
    if !csv_file.exists() {
        crate::throw_rerr!(
            NOT_FOUND,
            "Failed to open {} (error: not found)",
            csv_file.to_str().unwrap()
        );
    }

    if !csv_file.is_file() {
        crate::throw_rerr!(
            NOT_ALLOWED,
            "Failed to open {} (error: not a file)",
            csv_file.to_str().unwrap()
        );
    }

    let mut rdr = csv::Reader::from_path(&csv_file).map_err(|e| {
        new_custom_msg(
            NOT_ALLOWED,
            format!(
                "Failed to open {} (error:{})",
                csv_file.to_str().unwrap(),
                e
            ),
        )
    })?;

    let zones = common::detect_cpu();
    let mut map = HashMap::new();
    let mut out_map = HashMap::new();

    for res in rdr.deserialize() {
        let r: models::RAPLData = res.map_err(|e| {
            new_custom_msg(
                INVALID_VALUE,
                format!("Failed to deserialize RAPLData (error:{})", e),
            )
        })?;

        map.entry(r.zone_name.clone()).or_insert(vec![]).push(r);
    }

    for zone in zones {
        let zone_data = map.entry(zone.name.clone()).or_insert(vec![]);
        zone_data.remove(0);
        let data_len = zone_data.len();
        let mut delta_power_j_step = vec![];
        let mut watts_step = vec![];
        let mut avg_watts = vec![];

        for n in 0..data_len - 1 {
            delta_power_j_step.push(zone_data[n].delta_power_j);
            watts_step.push(zone_data[n].watt);
            avg_watts.push(zone_data[n].avg_watt);
        }

        out_map.insert(
            zone.name,
            models::IsolateData {
                delta_power_j: StatData {
                    min: delta_power_j_step.iter().cloned().fold(0. / 0., f64::min),
                    max: delta_power_j_step.iter().cloned().fold(0. / 0., f64::max),
                    avg: delta_power_j_step.iter().sum::<f64>() / data_len as f64,
                    total: delta_power_j_step.iter().sum(),
                },
                watt: StatData {
                    min: watts_step.iter().cloned().fold(0. / 0., f64::min),
                    max: watts_step.iter().cloned().fold(0. / 0., f64::max),
                    avg: watts_step.iter().sum::<f64>() / data_len as f64,
                    total: watts_step.iter().sum(),
                },
                avg_watt: StatData {
                    min: avg_watts.iter().cloned().fold(0. / 0., f64::min),
                    max: avg_watts.iter().cloned().fold(0. / 0., f64::max),
                    avg: avg_watts.iter().sum::<f64>() / data_len as f64,
                    total: avg_watts.iter().sum(),
                },
            },
        );
    }

    let filename = dir.cloned().unwrap_or_default().join(format!(
        "{}-isolate-{}.json",
        name,
        SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ));

    logger::log_isolate_data(filename, out_map);

    Ok(())
}

pub(crate) fn measure_isolate_data(
    poll_delay: u32,
    dir: Option<&PathBuf>,
    name: &String,
    mintues: u32,
) -> Result<(), RError> {
    let system_start_time = SystemTime::now();
    let csv_file = dir.unwrap_or(&PathBuf::new()).join(format!(
        "{}-isolate-{}.csv",
        name,
        system_start_time
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    ));

    let sleep = std::time::Duration::from_millis(poll_delay as u64);
    let mut zones = common::setup_rapl_data();
    assert!(zones.len() != 0);
    let start_time = Instant::now();
    let mut prev_time = start_time;
    #[allow(unused_assignments)]
    let mut now = start_time;
    let mintues = mintues.max(1);
    let total_num_sample = mintues * 60 * 1000 / poll_delay;
    let mut num_sample = 0;
    let run_time_limit = mintues as u64 * 60;

    loop {
        now = Instant::now();
        common::update_measurements(&mut zones, now, start_time, prev_time, &csv_file, None);
        prev_time = now;

        ncurses::clear();
        ncurses::addstr(format!("Samples ({}/{})\n", num_sample, total_num_sample).as_str());
        ncurses::refresh();
        num_sample += 1;

        if ncurses::getch() == common::KEY_CODE_EXIT {
            ncurses::endwin();
            break;
        }

        if common::should_terminate(run_time_limit, now, start_time) {
            common::terminate();
            break;
        }
        std::thread::sleep(sleep);
        // std::thread::sleep(std::time::Duration::from_secs(100000));
    }

    generate_isolate_data(dir, name, csv_file)
}

pub(crate) fn extract_data(
    dir: Option<&PathBuf>,
    name: &String,
    csv_file: PathBuf,
) -> Result<(), RError> {
    if !csv_file.exists() {
        crate::throw_rerr!(
            NOT_FOUND,
            "Failed to open {} (error: not found)",
            csv_file.to_str().unwrap()
        );
    }

    if !csv_file.is_file() {
        crate::throw_rerr!(
            NOT_ALLOWED,
            "Failed to open {} (error: not a file)",
            csv_file.to_str().unwrap()
        );
    }

    let mut rdr = csv::Reader::from_path(&csv_file).map_err(|e| {
        new_custom_msg(
            NOT_ALLOWED,
            format!(
                "Failed to open {} (error: {})",
                &csv_file.to_str().unwrap(),
                e
            ),
        )
    })?;

    let dir = match dir {
        Some(dir) => dir.to_owned(),
        None => PathBuf::new(),
    };
    let system_start_time = SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut map: HashMap<String, Vec<RAPLData>> = HashMap::new();
    for res in rdr.deserialize::<RAPLData>() {
        match res {
            Ok(r) => {
                map.entry(r.zone_name.clone()).or_insert(vec![]).push(r);
            }
            Err(e) => {
                crate::throw_rerr!(
                    NOT_ALLOWED,
                    "Failed to parse {} (error: {})",
                    csv_file.to_str().unwrap(),
                    e
                );
            }
        }
    }

    for (domain, datas) in map {
        let file_path = dir.join(format!(
            "{}-extract-{}-{}.csv",
            name, domain, system_start_time
        ));
        let mut wdr = csv::Writer::from_path(&file_path).map_err(|e| {
            new_custom_msg(
                NOT_ALLOWED,
                format!(
                    "Failed to open {} (error: {})",
                    &file_path.to_str().unwrap(),
                    e
                ),
            )
        })?;
        for data in datas {
            wdr.serialize(data).map_err(|e| {
                new_custom_msg(
                    NOT_ALLOWED,
                    format!(
                        "Failed to write {} (error: {})",
                        &file_path.to_str().unwrap(),
                        e
                    ),
                )
            })?;
        }
    }

    Ok(())
}
