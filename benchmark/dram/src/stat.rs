use std::{
    sync::{atomic::AtomicBool, Mutex},
    thread::JoinHandle,
};

use once_cell::sync::Lazy;

static RUNNING: AtomicBool = AtomicBool::new(false);
static THR: Lazy<Mutex<Option<JoinHandle<()>>>> = Lazy::new(|| Mutex::new(None));

pub fn start_stat(interval: u32) {
    let mut thr = THR.lock().unwrap();
    if thr.is_some() {
        return;
    }
    RUNNING.store(true, std::sync::atomic::Ordering::Relaxed);
    *thr = Some(std::thread::spawn(move || {
        log::info!("Start stat");
        let mut last_bytes_movement = 0;
        loop {
            if !RUNNING.load(std::sync::atomic::Ordering::Relaxed) {
                break;
            }

            let bytes = crate::BYTES_MOVEMENT.load(std::sync::atomic::Ordering::Relaxed);
            let diff_bits = (bytes - last_bytes_movement) * 8;
            last_bytes_movement = bytes;

            println!(
                "Bytes Movement Rate: {} Mbit/s",
                diff_bits as f64 / 1000000.0 / interval as f64
            );
            std::thread::sleep(std::time::Duration::from_secs(interval as u64));
        }
        log::info!("Stop stat");
    }));
}

pub fn stop_stat() {
    let mut thr = THR.lock().unwrap();
    if thr.is_none() {
        return;
    }

    RUNNING.store(false, std::sync::atomic::Ordering::Relaxed);

    thr.take().unwrap().join().unwrap();
}
