use crate::models;

use csv;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

pub(crate) fn log_poll_result(zone: &mut models::RAPLData, file_path: &PathBuf) {
    let file_path = PathBuf::from(file_path);

    if file_path.exists() {
        let file = OpenOptions::new()
            .write(true)
            .append(true)
            .open(&file_path)
            .unwrap();

        let mut wtr = csv::WriterBuilder::default()
            .has_headers(false)
            .from_writer(file);
        wtr.serialize(zone).expect("Failed to write to file");

        let mut perms = fs::metadata(&file_path).unwrap().permissions();
        perms.set_mode(0o666);
        fs::set_permissions(&file_path, perms).expect("Failed to set permissions for file");
    } else {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(&file_path)
            .unwrap();

        let mut wtr = csv::Writer::from_writer(file);
        wtr.serialize(zone).expect("Failed to write to file");
    }
}

pub(crate) fn log_isolate_data(file_name: PathBuf, map: HashMap<String, models::IsolateData>) {
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .open(file_name)
        .unwrap();
    let json = serde_json::to_string_pretty(&map).unwrap();

    file.write(json.as_bytes())
        .expect("Failed to write isolation data to file");
    drop(file);
}
