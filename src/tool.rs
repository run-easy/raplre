// struct MSRReader {
//     core: u32,
//     msr: std::fs::File,
// }

// impl MSRReader {
//     /// Open MSR file for read.
//     pub fn open(core: u32) -> Result<Self, RError> {
//         let path = format!("/dev/cpu/{}/msr", core);
//         let msr = std::fs::File::open(path).map_err(|e| match e.kind() {
//             std::io::ErrorKind::NotFound => new_custom_msg(NOT_FOUND, e.to_string()),
//             std::io::ErrorKind::PermissionDenied => new_custom_msg(NOT_ALLOWED, e.to_string()),
//             std::io::ErrorKind::InvalidInput => new_custom_msg(INVALID_VALUE, e.to_string()),
//             _ => unreachable!(),
//         })?;

//         Ok(Self {
//             core: core,
//             msr: msr,
//         })
//     }

//     pub fn read(&mut self, which: i64) -> Result<u64, RError> {
//         let mut data: u64 = 0;
//         unsafe {
//             if libc::pread(
//                 self.msr.as_raw_fd(),
//                 &mut data as *mut _ as *mut _,
//                 std::mem::size_of_val(&data),
//                 which,
//             ) as usize
//                 != std::mem::size_of_val(&data)
//             {
//                 throw_rerr!(IOERR, "Failed to read msr({}) at {}", self.core, which);
//             }
//         }

//         Ok(data)
//     }
// }

use crate::common;
use crate::error::*;

pub(crate) fn live_measurement(
    poll_delay: u32,
    run_time_limit: Option<u32>,
    name: String,
) -> Result<(), RError> {
    let sleep = std::time::Duration::from_micros(poll_delay as u64);
    let mut zones = common::setup_rapl_data();
    todo!()
}
