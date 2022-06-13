cfg_if::cfg_if! {
    if #[cfg(target_os = "windows")] {
        mod windows;
        use self::windows as imp;
    } else if #[cfg(target_os = "linux")] {
        mod linux;
        use self::linux as imp;
    } else {
        compile_error!("unsupported platform");
    }
}

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
        println!("Boot Time: {}", time::OffsetDateTime::from(boot_time));
    }
}
