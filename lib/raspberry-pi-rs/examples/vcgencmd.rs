///! A port of `vcgencmd` to Rust
///!
///! See:
///! * `https://chem.libretexts.org/Courses/Intercollegiate_Courses/Internet_of_Science_Things_(2020)/5%3A_Appendix_3%3A_General_Tasks/5.9%3A_Monitoring_your_Raspberry_Pi#:~:text=Using%20the%20vcgencmd%20command%20we,information%20about%20our%20Raspberry%20Pis.&text=According%20to%20Raspberry%20Pi%20Documentation,with%20a%20half%2Dfilled%20thermometer`
///! * `https://www.raspberrypi.com/documentation/computers/os.html#vcgencmd`
///! * `https://github.com/raspberrypi/userland/blob/6e8f786db223c2ab6eb9098a5cb0e5e1b25281cd/host_applications/linux/apps/gencmd/gencmd.c#L40-L53`
use std::process::ExitCode;

/// Ported from `https://github.com/raspberrypi/userland/blob/6e8f786db223c2ab6eb9098a5cb0e5e1b25281cd/host_applications/linux/apps/gencmd/gencmd.c#L40-L53`
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

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 1 {
        return ExitCode::from(0);
    }

    if args[1] == "-h" || args[1] == "--help" {
        show_usage();
        return ExitCode::from(0);
    }

    #[cfg(all(
        feature = "wrapper",
        all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
    ))]
    {
        use raspberry_pi::RaspberryPi;

        let mut raspberrypi =
            unsafe { RaspberryPi::new().expect("failed to load raspberry pi libraries") };

        raspberrypi.vcos_init().expect("failed to init vcos");
        raspberrypi.bcm_host_init();

        let command = args[1..].join(" ");
        if let Err(e) = raspberrypi.vc_gencmd_send(command) {
            println!("vc_gencmd_send returned {:?}", e);
        }
        match raspberrypi
            .vc_gencmd_read_response()
            .map(|response| response.into_string())
        {
            Ok(Ok(response)) => {
                if !response.is_empty() {
                    if response.ends_with('\n') {
                        println!("{}", response);
                    } else if response.starts_with("error=") {
                        eprintln!("{}", response);
                        if response == "error=1 error_msg=\"Command not registered\"" {
                            eprintln!("Use 'vcgencmd commands' to get a list of commands");
                        }

                        // TODO: Return nicely somehow
                        std::process::exit(-2);
                    } else {
                        println!("{}", response);
                    }
                }
            }
            Ok(Err(e)) => {
                println!("response is not valid utf8: {}", e);
            }
            Err(e) => {
                println!("vc_gencmd_read_response returned {:?}", e);
            }
        }

        unsafe {
            raspberrypi.vcos_deinit();
            raspberrypi.bcm_host_deinit().expect("failed to deinit");
        }

        return ExitCode::from(0);
    }

    #[cfg(not(all(
        feature = "wrapper",
        all(any(target_arch = "arm", target_arch = "aarch64"), target_os = "linux")
    )))]
    {
        panic!("this example will currently not work without the `wrapper` feature and will not work on platforms that are not arm linux");
    }
}
