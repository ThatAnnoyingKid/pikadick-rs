use crate::checks::ENABLED_CHECK;
use anyhow::Context as _;
use heim::units::{
    frequency::{
        gigahertz,
        hertz,
    },
    Frequency,
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
use std::time::{
    Duration,
    Instant,
};
use systemstat::{
    platform::common::Platform,
    System,
};
use time::format_description::well_known::Rfc2822;
use tracing::warn;
use uom::{
    fmt::DisplayStyle,
    si::f32::Frequency as FrequencyF32,
};

const BYTES_IN_GB_F64: f64 = 1_000_000_000_f64;

fn fmt_cpu_frequency(freq: &Frequency) -> String {
    let fmt_args = FrequencyF32::format_args(gigahertz, DisplayStyle::Abbreviation);
    let freq = FrequencyF32::new::<hertz>(freq.get::<hertz>() as f32);

    format!("{:.2}", fmt_args.with(freq))
}

async fn get_cpu_usage() -> Result<f32, heim::Error> {
    let start = heim::cpu::usage().await?;
    tokio::time::sleep(Duration::from_secs(1)).await;
    let end = heim::cpu::usage().await?;

    Ok((end - start).get::<heim::units::ratio::percent>())
}

#[command]
#[description("Get System Stats")]
#[bucket("system")]
#[checks(Enabled)]
async fn system(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let start = Instant::now();

    let profile_avatar_url = ctx.cache.current_user().avatar_url();

    // Start Legacy data gathering
    let sys = System::new();

    let cpu_temp = match sys.cpu_temp() {
        Ok(cpu_temp) => Some(cpu_temp),
        Err(e) => {
            warn!("Failed to get cpu temp: {}", e);
            None
        }
    };

    // End Legacy data gathering

    let boot_time = match pikadick_system_info::get_boot_time()
        .context("failed to get boot time")
        .map(time::OffsetDateTime::from)
        .and_then(|boot_time| {
            boot_time
                .format(&Rfc2822)
                .context("failed to format boot time date")
        }) {
        Ok(boot_time) => Some(boot_time),
        Err(e) => {
            warn!("{:?}", e);
            None
        }
    };

    let uptime = match pikadick_system_info::get_uptime().context("failed to get uptime") {
        Ok(uptime) => Some(uptime),
        Err(e) => {
            warn!("{:?}", e);
            None
        }
    };

    let hostname = match pikadick_system_info::get_hostname().context("failed to get hostname") {
        Ok(hostname) => Some(hostname),
        Err(e) => {
            warn!("{:?}", e);
            None
        }
    };

    let architecture =
        match pikadick_system_info::get_architecture().context("failed to get architecture") {
            Ok(architecture) => Some(
                architecture
                    .map(|architecture| architecture.as_str())
                    .unwrap_or("unknown"),
            ),
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

    let system_name =
        match pikadick_system_info::get_system_name().context("failed to get system name") {
            Ok(system_name) => system_name,
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

    let system_version =
        match pikadick_system_info::get_system_version().context("failed to get system version") {
            Ok(system_name) => Some(system_name),
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

    let total_memory =
        match pikadick_system_info::get_total_memory().context("failed to get total memory") {
            Ok(memory) => Some(memory),
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

    let available_memory = match pikadick_system_info::get_available_memory()
        .context("failed to get available memory")
    {
        Ok(memory) => Some(memory),
        Err(e) => {
            warn!("{:?}", e);
            None
        }
    };

    let total_swap =
        match pikadick_system_info::get_total_swap().context("failed to get total swap") {
            Ok(memory) => Some(memory),
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

    let available_swap =
        match pikadick_system_info::get_available_swap().context("failed to get available swap") {
            Ok(memory) => Some(memory),
            Err(e) => {
                warn!("{:?}", e);
                None
            }
        };

    let cpu_frequency = match heim::cpu::frequency().await {
        Ok(cpu_frequency) => Some(cpu_frequency),
        Err(e) => {
            warn!("Failed to get cpu frequency: {}", e);
            None
        }
    };

    let cpu_logical_count = match heim::cpu::logical_count().await {
        Ok(cpu_logical_count) => Some(cpu_logical_count),
        Err(e) => {
            warn!("Failed to get logical cpu count: {}", e);
            None
        }
    };

    let cpu_physical_count = match heim::cpu::physical_count().await {
        Ok(cpu_physical_count) => cpu_physical_count, // This returns an option, so we return it here to flatten it.
        Err(e) => {
            warn!("Failed to get physical cpu count: {}", e);
            None
        }
    };

    let virtualization = heim::virt::detect().await;

    let cpu_usage = match get_cpu_usage().await {
        Ok(usage) => Some(usage),
        Err(e) => {
            warn!("Failed to get cpu usage: {}", e);
            None
        }
    };

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

                if let Some(icon) = profile_avatar_url {
                    e.thumbnail(&icon);
                }

                if let Some(hostname) = hostname {
                    e.field("Hostname", hostname, true);
                }

                if let Some(system_name) = system_name {
                    e.field("OS", system_name, true);
                }

                if let Some(system_version) = system_version {
                    e.field("OS Version", system_version, true);
                }

                if let Some(architecture) = architecture {
                    e.field("Architecture", architecture, true);
                }

                if let Some(boot_time) = boot_time {
                    e.field("Boot Time", boot_time, true);
                }

                if let Some(uptime) = uptime {
                    let raw_secs = uptime.as_secs();

                    let days = raw_secs / (60 * 60 * 24);
                    let hours = (raw_secs % (60 * 60 * 24)) / (60 * 60);
                    let minutes = (raw_secs % (60 * 60)) / 60;
                    let seconds = raw_secs % 60;

                    let mut value = String::with_capacity(64);
                    if days != 0 {
                        value.push_str(itoa::Buffer::new().format(days));
                        value.push_str(" day");
                        if days > 1 {
                            value.push('s');
                        }
                    }

                    if hours != 0 {
                        value.push(' ');

                        value.push_str(itoa::Buffer::new().format(hours));
                        value.push_str(" hour");
                        if hours > 1 {
                            value.push('s');
                        }
                    }

                    if minutes != 0 {
                        value.push(' ');

                        value.push_str(itoa::Buffer::new().format(minutes));
                        value.push_str(" minute");
                        if minutes > 1 {
                            value.push('s');
                        }
                    }

                    if seconds != 0 {
                        value.push(' ');

                        value.push_str(itoa::Buffer::new().format(seconds));
                        value.push_str(" second");
                        if seconds > 1 {
                            value.push('s');
                        }
                    }

                    e.field("Uptime", value, true);
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
                            &format!("{} logical, {} physical", logical_count, physical_count),
                            true,
                        );
                    }
                    (Some(logical_count), None) => {
                        e.field(
                            "Cpu Core Count",
                            &format!("{} logical", logical_count),
                            true,
                        );
                    }
                    (None, Some(physical_count)) => {
                        e.field(
                            "Cpu Core Count",
                            &format!("{} physical", physical_count),
                            true,
                        );
                    }
                    (None, None) => {}
                }

                if let (Some(total_memory), Some(available_memory)) =
                    (total_memory, available_memory)
                {
                    e.field(
                        "Memory Usage",
                        format!(
                            "{:.2} GB / {:.2} GB",
                            (total_memory - available_memory) as f64 / BYTES_IN_GB_F64,
                            total_memory as f64 / BYTES_IN_GB_F64,
                        ),
                        true,
                    );
                }

                if let (Some(total_swap), Some(available_swap)) = (total_swap, available_swap) {
                    e.field(
                        "Swap Usage",
                        format!(
                            "{:.2} GB / {:.2} GB",
                            (total_swap - available_swap) as f64 / BYTES_IN_GB_F64,
                            total_swap as f64 / BYTES_IN_GB_F64,
                        ),
                        true,
                    );
                }

                let virtualization_field = match virtualization.as_ref() {
                    Some(virtualization) => virtualization.as_str(),
                    None => "None",
                };
                e.field("Virtualization", virtualization_field, true);

                if let (Some(cpu_usage), Some(cpu_logical_count)) = (cpu_usage, cpu_logical_count) {
                    e.field(
                        "Cpu Usage",
                        &format!("{:.2}%", cpu_usage / (cpu_logical_count as f32)),
                        true,
                    );
                }

                /////////////////////////////////////////////////////////////////////////////////////
                // Legacy (These functions from systemstat have no direct replacement in heim yet) //
                /////////////////////////////////////////////////////////////////////////////////////

                // This does not work on Windows
                // TODO: This can probably be replaced with temprature readings from heim.
                // It doesn't support Windows, but this never worked there anyways as Windows has no simple way to get temps
                if let Some(cpu_temp) = cpu_temp {
                    e.field("Cpu Temp", &format!("{} °C", cpu_temp), true);
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
