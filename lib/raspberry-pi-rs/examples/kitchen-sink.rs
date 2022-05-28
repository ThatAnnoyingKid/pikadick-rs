use raspberrypi::RaspberryPi;

fn main() {
    let mut raspberrypi =
        unsafe { RaspberryPi::new().expect("failed to load raspberry pi libraries") };

    raspberrypi.bcm_host_init();
    let model_type = raspberrypi
        .get_model_type()
        .expect("failed to get model type");
    let display_size = raspberrypi.graphics_get_display_size(0);

    println!("Model Type: {:?}", model_type);
    println!("Display Size: {:?}", display_size);
    println!(
        "Is Pi 4?: {}",
        raspberrypi
            .is_model_pi4()
            .expect("failed to check if model is a pi4")
    );
    println!(
        "Is FKMS Active?: {}",
        raspberrypi
            .is_fkms_active()
            .expect("failed to check if fkms is active")
    );
    println!(
        "Is KMS Active?: {}",
        raspberrypi
            .is_kms_active()
            .expect("failed to check if kms is active")
    );
    println!(
        "Processor Id: {:?}",
        raspberrypi
            .get_processor_id()
            .expect("failed to get processor id")
    );

    unsafe {
        raspberrypi.bcm_host_deinit().expect("failed to deinit");
    }
}
