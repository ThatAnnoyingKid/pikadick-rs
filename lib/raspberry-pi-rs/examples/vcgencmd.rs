use raspberry_pi::RaspberryPi;

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
