#[cfg(all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux"))]
fn main() {
    println!("# Rust");
    println!(
        "Model Type: {:?}",
        raspberry_pi::bcm_host::get_model_type().expect("failed to get model type")
    );
    println!(
        "Is Pi 4?: {}",
        raspberry_pi::bcm_host::is_model_pi4().expect("failed to check if the host is a pi 4")
    );
    println!(
        "Is FKMS Active?: {}",
        raspberry_pi::bcm_host::is_fkms_active().expect("failed to check if fkms is active")
    );
    println!(
        "Is KMS Active?: {}",
        raspberry_pi::bcm_host::is_kms_active().expect("failed to check if kms is active")
    );
    println!(
        "Processor Id: {:?}",
        raspberry_pi::bcm_host::get_processor_id().expect("failed to get processor id")
    );
    println!();

    #[cfg(feature = "wrapper")]
    {
        use raspberry_pi::RaspberryPi;

        // Native
        let mut raspberrypi =
            unsafe { RaspberryPi::new().expect("failed to load raspberry pi libraries") };

        raspberrypi.bcm_host_init();
        let model_type = raspberrypi
            .get_model_type()
            .expect("failed to get model type");
        let display_size = raspberrypi.graphics_get_display_size(0);

        println!("# Native");
        println!("Model Type: {:?}", model_type);
        println!("Display Size: {:?}", display_size);
        println!("Is Pi 4?: {}", raspberrypi.is_model_pi4());
        println!("Is FKMS Active?: {}", raspberrypi.is_fkms_active());
        println!("Is KMS Active?: {}", raspberrypi.is_kms_active());
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
}

#[cfg(not(all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")))]
fn main() {
    panic!("this example only works for arm on linux");
}
