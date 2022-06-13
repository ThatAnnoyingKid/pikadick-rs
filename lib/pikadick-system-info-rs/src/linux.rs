use super::Error;
use std::time::SystemTime;

pub fn get_boot_time() -> Result<SystemTime, Error> {
    let sysinfo = nix::sys::sysinfo::sysinfo().map_err(std::io::Error::from)?;
    Ok(SystemTime::now() - sysinfo.uptime())
}
