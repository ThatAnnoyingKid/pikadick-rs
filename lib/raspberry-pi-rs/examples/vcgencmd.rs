#[cfg(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
))]
use raspberry_pi::RaspberryPi;
#[cfg(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
))]
use std::process::ExitCode;

/// Ported from `https://github.com/raspberrypi/userland/blob/6e8f786db223c2ab6eb9098a5cb0e5e1b25281cd/host_applications/linux/apps/gencmd/gencmd.c#L40-L53`
#[cfg(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
))]
fn show_usage() {
    println!("Usage: vcgencmd [-t] command");
    println!("Send a command to the VideoCore and print the result.\n");
    println!("  -t          Time how long the command takes to complete");
    println!("  -h, --help  Show this information\n");
    println!("Use the command 'vcgencmd commands' to get a list of available commands\n");
    println!("Exit status:");
    println!("   0    command completed successfully");
    println!("  -1    problem with VCHI");
    println!("  -2    VideoCore returned an error\n");
    println!("For further documentation please see");
    println!("https://www.raspberrypi.org/documentation/computers/os.html#vcgencmd\n");
}

#[cfg(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
))]
fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        return ExitCode::from(0);
    }

    if args[1] == "-h" || args[1] == "--help" {
        show_usage();
        return ExitCode::from(0);
    }

    let mut raspberrypi =
        unsafe { RaspberryPi::new().expect("failed to load raspberry pi libraries") };

    raspberrypi.bcm_host_init();

    let command = args[1..].join(" ");
    raspberrypi
        .vc_gencmd_send(command)
        .expect("failed to measure temp");
    let response = raspberrypi
        .vc_gencmd_read_response()
        .expect("failed to read response")
        .into_string()
        .expect("response is not valid utf8");
    println!("{}", response);

    unsafe {
        raspberrypi.bcm_host_deinit().expect("failed to deinit");
    }

    return ExitCode::from(0);
}

#[cfg(not(all(
    feature = "wrapper",
    all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
)))]
fn main() {
    panic!("this example will currently not work without the `wrapper` feature and will not work on platforms that are not arm linux");
}
