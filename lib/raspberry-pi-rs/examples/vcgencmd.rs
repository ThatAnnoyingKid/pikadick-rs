#[cfg(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
))]
use raspberry_pi::RaspberryPi;

#[cfg(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
))]
fn main() {
    let mut raspberrypi =
        unsafe { RaspberryPi::new().expect("failed to load raspberry pi libraries") };

    raspberrypi.bcm_host_init();

    raspberrypi
        .vc_gencmd_send("measure_temp")
        .expect("failed to measure temp");
    let raw_response = raspberrypi
        .vc_gencmd_read_response()
        .expect("failed to read response");
    println!(
        "{:?}",
        raw_response
            .into_string()
            .expect("response is not valid utf8")
    );

    unsafe {
        raspberrypi.bcm_host_deinit().expect("failed to deinit");
    }
}

#[cfg(not(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
)))]
fn main() {
    panic!("this example will currently not work without the `wrapper` feature and will not work on platforms that are not arm linux");
}
