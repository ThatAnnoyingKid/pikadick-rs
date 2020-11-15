use crate::{
    checks::ENABLED_CHECK,
    ClientDataKey,
};
use chrono::DateTime;
use heim::{
    memory::{
        Memory,
        Swap,
    },
    units::{
        frequency::{
            gigahertz,
            hertz,
        },
        information::{
            byte,
            gigabyte,
        },
        time::nanosecond,
        Frequency,
    },
};
use serenity::{
    framework::standard::{
        macros::command,
        Args,
        CommandResult,
    },
    model::prelude::*,
    prelude::*,
    utils::Colour,
};
use slog::warn;
use std::time::{
    Duration,
    Instant,
    UNIX_EPOCH,
};
use systemstat::{
    platform::common::Platform,
    System,
};
use uom::{
    fmt::DisplayStyle,
    si::f32::{
        Frequency as FrequencyF32,
        Information as InformationF32,
    },
};

fn epoch_nanos_to_local_datetime(nanos: u64) -> DateTime<chrono::Local> {
    DateTime::from(UNIX_EPOCH + Duration::from_nanos(nanos))
}

fn fmt_uptime(uptime: Duration) -> String {
    let raw_secs = uptime.as_secs();

    let days = raw_secs / (60 * 60 * 24);
    let hours = (raw_secs % (60 * 60 * 24)) / (60 * 60);
    let minutes = (raw_secs % (60 * 60)) / 60;
    let secs = raw_secs % 60;

    format!(
        "{} days {} hours {} minutes {} seconds",
        days, hours, minutes, secs
    )
}

fn fmt_memory(memory: &Memory) -> String {
    let fmt_args = InformationF32::format_args(gigabyte, DisplayStyle::Abbreviation);

    let avail_mem = InformationF32::new::<byte>(memory.available().get::<byte>() as f32);
    let total_mem = InformationF32::new::<byte>(memory.total().get::<byte>() as f32);
    let used_mem = total_mem - avail_mem;

    format!(
        "{:.2} / {:.2}",
        fmt_args.with(used_mem),
        fmt_args.with(total_mem),
    )
}

fn fmt_swap(swap: &Swap) -> String {
    let fmt_args = InformationF32::format_args(gigabyte, DisplayStyle::Abbreviation);

    let used = InformationF32::new::<byte>(swap.used().get::<byte>() as f32);
    let total = InformationF32::new::<byte>(swap.total().get::<byte>() as f32);

    format!("{:.2} / {:.2}", fmt_args.with(used), fmt_args.with(total),)
}

fn fmt_cpu_frequency(freq: &Frequency) -> String {
    let fmt_args = FrequencyF32::format_args(gigahertz, DisplayStyle::Abbreviation);
    let freq = FrequencyF32::new::<hertz>(freq.get::<hertz>() as f32);

    format!("{:.2}", fmt_args.with(freq))
}

#[command]
#[description("Get System Stats")]
#[bucket("system")]
#[checks(Enabled)]
async fn system(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let data_lock = ctx.data.read().await;
    let client_data = data_lock.get::<ClientDataKey>().unwrap();
    let logger = client_data.logger.clone();
    drop(data_lock);

    let start = Instant::now();

    let profile = ctx.http.get_current_user().await?;

    // Start Legacy data gathering
    let sys = System::new();
    let cpu_load = match get_cpu_load_aggregate(&sys).await {
        Ok(cpu_load) => Some(cpu_load),
        Err(e) => {
            warn!(logger, "Failed to get platform cpu load: {}", e);
            None
        }
    };

    let cpu_temp = match sys.cpu_temp() {
        Ok(cpu_temp) => Some(cpu_temp),
        Err(e) => {
            warn!(logger, "Failed to get cpu temp: {}", e);
            None
        }
    };
    // End Legacy data gathering

    let platform = match heim::host::platform().await {
        Ok(platform) => Some(platform),
        Err(e) => {
            warn!(logger, "Failed to get platform info: {}", e);
            None
        }
    };

    let boot_time = match heim::host::boot_time().await {
        Ok(boot_time) => Some(epoch_nanos_to_local_datetime(
            boot_time.get::<nanosecond>() as u64
        )),
        Err(e) => {
            warn!(logger, "Failed to get boot time: {}", e);
            None
        }
    };

    let uptime = match heim::host::uptime().await {
        Ok(uptime) => Some(Duration::from_nanos(uptime.get::<nanosecond>() as u64)),
        Err(e) => {
            warn!(logger, "Failed to get uptime: {}", e);
            None
        }
    };

    let cpu_frequency = match heim::cpu::frequency().await {
        Ok(cpu_frequency) => Some(cpu_frequency),
        Err(e) => {
            warn!(logger, "Failed to get cpu frequency: {}", e);
            None
        }
    };

    let cpu_logical_count = match heim::cpu::logical_count().await {
        Ok(cpu_logical_count) => Some(cpu_logical_count),
        Err(e) => {
            warn!(logger, "Failed to get logical cpu count: {}", e);
            None
        }
    };

    let cpu_physical_count = match heim::cpu::physical_count().await {
        Ok(cpu_physical_count) => cpu_physical_count, // This returns an option, so we return it here to flatten it.
        Err(e) => {
            warn!(logger, "Failed to get physical cpu count: {}", e);
            None
        }
    };

    let memory = match heim::memory::memory().await {
        Ok(memory) => Some(memory),
        Err(e) => {
            warn!(logger, "Failed to get memory usage: {}", e);
            None
        }
    };

    let swap = match heim::memory::swap().await {
        Ok(swap) => Some(swap),
        Err(e) => {
            warn!(logger, "Failed to get swap usage: {}", e);
            None
        }
    };

    let virtualization = heim::virt::detect().await;

    let data_retrieval_time = Instant::now() - start;

    // Start WIP

    // Reports Cpu time since boot.
    // let cpu_time = heim::cpu::time().await.unwrap();

    // Reports some cpu stats
    // let cpu_stats = heim::cpu::stats().await.unwrap();

    // Reports temps from all sensors
    // let temperatures = heim::sensors::temperatures().collect::<Vec<_>>().await;

    // End WIP

    msg.channel_id
        .send_message(&ctx.http, |m| {
            m.embed(|e| {
                e.title("System Status");
                e.color(Colour::from_rgb(255, 0, 0));

                if let Some(icon) = profile.avatar_url() {
                    e.thumbnail(icon);
                }

                if let Some(platform) = platform {
                    e.field("Hostname", platform.hostname(), true);

                    e.field(
                        "OS",
                        format!(
                            "{} {} (version {})",
                            platform.system(),
                            platform.release(),
                            platform.version()
                        ),
                        true,
                    );

                    e.field("Arch", platform.architecture().as_str(), true);
                }

                if let Some(boot_time) = boot_time {
                    e.field("Boot Time", boot_time.to_rfc2822(), true);
                }

                if let Some(uptime) = uptime {
                    e.field("Uptime", fmt_uptime(uptime), true);
                }

                // Currently reports incorrectly on Windows
                if let Some(cpu_frequency) = cpu_frequency {
                    e.field(
                        "Cpu Freq",
                        fmt_cpu_frequency(&cpu_frequency.current()),
                        true,
                    );

                    if let Some(min_cpu_frequency) = cpu_frequency.min() {
                        e.field("Min Cpu Freq", fmt_cpu_frequency(&min_cpu_frequency), true);
                    }

                    if let Some(max_cpu_frequency) = cpu_frequency.max() {
                        e.field("Max Cpu Freq", fmt_cpu_frequency(&max_cpu_frequency), true);
                    }
                }

                match (cpu_logical_count, cpu_physical_count) {
                    (Some(logical_count), Some(physical_count)) => {
                        e.field(
                            "Cpu Core Count",
                            format!("{} logical, {} physical", logical_count, physical_count),
                            true,
                        );
                    }
                    (Some(logical_count), None) => {
                        e.field("Cpu Core Count", format!("{} logical", logical_count), true);
                    }
                    (None, Some(physical_count)) => {
                        e.field(
                            "Cpu Core Count",
                            format!("{} physical", physical_count),
                            true,
                        );
                    }
                    (None, None) => {}
                }

                if let Some(memory) = memory {
                    e.field("Memory Usage", fmt_memory(&memory), true);
                }

                if let Some(swap) = swap {
                    e.field("Swap Usage", fmt_swap(&swap), true);
                }

                let virtualization_field = match virtualization.as_ref() {
                    Some(virtualization) => virtualization.as_str(),
                    None => "None",
                };
                e.field("Virtualization", virtualization_field, true);

                /////////////////////////////////////////////////////////////////////////////////////
                // Legacy (These functions from systemstat have no direct replacement in heim yet) //
                /////////////////////////////////////////////////////////////////////////////////////

                // This does not work on Windows
                // TODO: This can probably be replaced with temprature readings from heim.
                // It doesn't support Windows, but this never worked there anyways as Windows has no simple way to get temps
                if let Some(cpu_temp) = cpu_temp {
                    e.field("Cpu Temp", format!("{} °C", cpu_temp), true);
                }

                // This is wack on Windows with readings not always adding to 100%
                if let Some(cpu_load) = cpu_load {
                    e.field(
                        "Cpu User %",
                        &format!("{:.2}%", cpu_load.user * 100.0),
                        true,
                    );
                    e.field(
                        "Cpu Nice %",
                        &format!("{:.2}%", cpu_load.nice * 100.0),
                        true,
                    );
                    e.field(
                        "Cpu System %",
                        &format!("{:.2}%", cpu_load.system * 100.0),
                        true,
                    );
                    e.field(
                        "Cpu Interrupt %",
                        &format!("{:.2}%", cpu_load.interrupt * 100.0),
                        true,
                    );
                    e.field(
                        "Cpu Idle %",
                        &format!("{:.2}%", cpu_load.idle * 100.0),
                        true,
                    );
                }

                e.footer(|f| {
                    f.text(format!(
                        "Retrieved system data in {:.2} second(s)",
                        data_retrieval_time.as_secs_f32()
                    ));

                    f
                });

                e
            })
        })
        .await?;

    Ok(())
}

async fn get_cpu_load_aggregate(sys: &System) -> Result<systemstat::data::CPULoad, std::io::Error> {
    let cpu_load = sys.cpu_load_aggregate()?;
    tokio::time::delay_for(std::time::Duration::from_secs(1)).await;
    cpu_load.done()
}
