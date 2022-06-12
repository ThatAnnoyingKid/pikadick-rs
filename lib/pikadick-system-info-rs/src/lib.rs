#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
use self::windows as imp;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
use self::linux as imp;

use std::time::SystemTime;

/// Get the boot time.
pub fn get_boot_time() -> SystemTime {
    imp::get_boot_time()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boot_time() {
        let boot_time = get_boot_time();
        dbg!(&boot_time);
    }
}
