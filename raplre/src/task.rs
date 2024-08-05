use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::Receiver,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::{common, models::IsolateData};

pub(crate) fn spawn_measurement_thread(
    start_time: Instant,
    recv: Receiver<i8>,
    poll_delay: u32,
    file_name: PathBuf,
    smooth: bool,
    isolate_map: Option<HashMap<String, IsolateData>>,
) -> JoinHandle<()> {
    let thr = thread::spawn(move || {
        let mut tzones = common::setup_rapl_data().to_owned();
        let mut prev_time = start_time;
        // reassign locally - unsafe otherwise
        let trecv = recv;
        let mut run = true;
        #[allow(unused_assignments)]
        let mut now = Instant::now();
        let duration = Duration::from_millis(poll_delay as u64);

        while run {
            now = Instant::now();
            common::update_measurements(
                &mut tzones,
                now,
                start_time,
                prev_time,
                &file_name,
                isolate_map.as_ref(),
                smooth,
            );
            prev_time = now;

            match trecv.recv_timeout(duration) {
                Ok(msg) => {
                    if msg == common::THREAD_KILL {
                        let now = Instant::now();
                        let _ = common::update_measurements(
                            &mut tzones,
                            now,
                            start_time,
                            prev_time,
                            &file_name,
                            isolate_map.as_ref(),
                            false,
                        );
                        run = false;
                    }
                }
                Err(_) => {}
            }
        }
    });

    return thr;
}
