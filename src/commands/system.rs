use crate::{
    PoiseContext,
    PoiseError,
};
use anyhow::Context as _;
use heim::units::{
    Frequency,
    frequency::{
        gigahertz,
        hertz,
    },
};
use poise::CreateReply;
use serenity::{
    builder::{
        CreateEmbed,
        CreateEmbedFooter,
    },
    model::colour::Colour,
};
use std::time::{
    Duration,
    Instant,
    SystemTime,
};
use systemstat::{
    System,
    platform::common::Platform,
};
use tracing::warn;
use uom::{
    fmt::DisplayStyle,
    si::f32::Frequency as FrequencyF32,
};

const BYTES_IN_GB_F64: f64 = 1_000_000_000_f64;

fn fmt_cpu_frequency(freq: Frequency) -> String {
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

fn format_boot_time(value: SystemTime) -> anyhow::Result<String> {
    let timestamp = jiff::Timestamp::try_from(value).context("Failed to make timestamp")?;
    Ok(timestamp.strftime("%e %b %Y, %I:%M:%S %p UTC").to_string())
}

fn format_uptime(value: Duration) -> anyhow::Result<String> {
    let printer = jiff::fmt::friendly::SpanPrinter::new()
        .spacing(jiff::fmt::friendly::Spacing::BetweenUnitsAndDesignators)
        .designator(jiff::fmt::friendly::Designator::Verbose);

    let now = jiff::Timestamp::now().to_zoned(jiff::tz::TimeZone::UTC);
    let span = jiff::Span::try_from(value).context("Failed to create span")?;
    let round_options = jiff::SpanRound::new()
        .smallest(jiff::Unit::Second)
        .largest(jiff::Unit::Year)
        .relative(&now);
    let span_rounded = span.round(round_options).context("Failed to round")?;

    Ok(printer.span_to_string(&span_rounded))
}

#[derive(Debug)]
struct SystemStats {
    hostname: Option<String>,
    system_name: Option<String>,
    system_version: Option<String>,
    architecture: Option<&'static str>,
    boot_time: Option<String>,
    uptime: Option<String>,
}

impl SystemStats {
    fn new() -> Self {
        let cache_context = pikadick_system_info::CacheContext::new();

        let hostname = match cache_context
            .get_hostname()
            .context("Failed to get hostname")
        {
            Ok(hostname) => Some(hostname),
            Err(error) => {
                warn!("{error:?}");
                None
            }
        };

        let system_name = match cache_context
            .get_system_name()
            .context("Failed to get system name")
        {
            Ok(system_name) => system_name,
            Err(error) => {
                warn!("{error:?}");
                None
            }
        };

        let system_version = match cache_context
            .get_system_version()
            .context("Failed to get system version")
        {
            Ok(system_name) => Some(system_name),
            Err(error) => {
                warn!("{error:?}");
                None
            }
        };

        let architecture = match cache_context
            .get_architecture()
            .context("Failed to get architecture")
        {
            Ok(architecture) => Some(
                architecture
                    .map(|architecture| architecture.as_str())
                    .unwrap_or("Unknown"),
            ),
            Err(error) => {
                warn!("{error:?}");
                None
            }
        };

        let boot_time = match cache_context
            .get_boot_time()
            .context("Failed to get boot time")
            .and_then(format_boot_time)
        {
            Ok(boot_time) => Some(boot_time),
            Err(error) => {
                warn!("{error:?}");
                None
            }
        };

        let uptime = match cache_context
            .get_uptime()
            .context("Failed to get uptime")
            .and_then(format_uptime)
        {
            Ok(uptime) => Some(uptime),
            Err(error) => {
                warn!("{error:?}");
                None
            }
        };

        Self {
            hostname,
            system_name,
            system_version,
            architecture,
            boot_time,
            uptime,
        }
    }

    fn populate_embed(&self, mut create_embed: CreateEmbed) -> CreateEmbed {
        let fields = [
            ("Hostname", self.hostname.as_deref()),
            ("OS", self.system_name.as_deref()),
            ("OS Version", self.system_version.as_deref()),
            ("Architecture", self.architecture),
            ("Boot Time", self.boot_time.as_deref()),
            ("Uptime", self.uptime.as_deref()),
        ];

        for (key, value) in fields {
            if let Some(value) = value {
                create_embed = create_embed.field(key, value, true);
            }
        }

        create_embed
    }
}

#[poise::command(slash_command, description_localized("en-US", "Get system stats."))]
pub async fn system(ctx: PoiseContext<'_>) -> Result<(), PoiseError> {
    ctx.defer().await?;

    let start = Instant::now();

    let profile_avatar_url = ctx.cache().current_user().avatar_url();

    // Start Legacy data gathering
    let sys = System::new();

    let cpu_temp = match sys.cpu_temp() {
        Ok(cpu_temp) => Some(cpu_temp),
        Err(error) => {
            warn!("Failed to get cpu temp: {error}");
            None
        }
    };

    // End Legacy data gathering
    let cache_context = pikadick_system_info::CacheContext::new();

    let total_memory = match cache_context
        .get_total_memory()
        .context("Failed to get total memory")
    {
        Ok(memory) => Some(memory),
        Err(error) => {
            warn!("{error:?}");
            None
        }
    };

    let available_memory = match cache_context
        .get_available_memory()
        .context("Failed to get available memory")
    {
        Ok(memory) => Some(memory),
        Err(error) => {
            warn!("{error:?}");
            None
        }
    };

    let total_swap = match cache_context
        .get_total_swap()
        .context("Failed to get total swap")
    {
        Ok(memory) => Some(memory),
        Err(error) => {
            warn!("{error:?}");
            None
        }
    };

    let available_swap = match cache_context
        .get_available_swap()
        .context("Failed to get available swap")
    {
        Ok(memory) => Some(memory),
        Err(error) => {
            warn!("{error:?}");
            None
        }
    };

    let cpu_logical_count = match cache_context
        .count_logical_cpus()
        .context("Failed to count number of logical cpus")
    {
        Ok(cpu_logical_count) => Some(cpu_logical_count),
        Err(error) => {
            warn!("Failed to get logical cpu count: {error}");
            None
        }
    };

    let cpu_frequency = match heim::cpu::frequency().await {
        Ok(cpu_frequency) => Some(cpu_frequency),
        Err(error) => {
            warn!("Failed to get cpu frequency: {error}");
            None
        }
    };

    let cpu_physical_count = match heim::cpu::physical_count().await {
        Ok(cpu_physical_count) => cpu_physical_count, // This returns an option, so we return it here to flatten it.
        Err(error) => {
            warn!("Failed to get physical cpu count: {error}");
            None
        }
    };

    let virtualization = heim::virt::detect().await;

    let cpu_usage = match get_cpu_usage().await {
        Ok(usage) => Some(usage),
        Err(error) => {
            warn!("Failed to get cpu usage: {error}");
            None
        }
    };

    let data_retrieval_time = Instant::now() - start;

    // Start WIP

    // Reports Cpu time since boot.
    // let cpu_time = heim::cpu::time().await.unwrap();

    // Reports temps from all sensors
    // let temperatures = heim::sensors::temperatures().collect::<Vec<_>>().await;

    // End WIP

    let system_stats = SystemStats::new();

    let mut create_embed = CreateEmbed::new()
        .title("System Status")
        .color(Colour::from_rgb(255, 0, 0));
    if let Some(icon) = profile_avatar_url {
        create_embed = create_embed.thumbnail(icon);
    }
    create_embed = system_stats.populate_embed(create_embed);

    // Currently reports incorrectly on Windows
    if let Some(cpu_frequency) = cpu_frequency {
        create_embed =
            create_embed.field("Cpu Freq", fmt_cpu_frequency(cpu_frequency.current()), true);

        if let Some(min_cpu_frequency) = cpu_frequency.min() {
            create_embed =
                create_embed.field("Min Cpu Freq", fmt_cpu_frequency(min_cpu_frequency), true);
        }

        if let Some(max_cpu_frequency) = cpu_frequency.max() {
            create_embed =
                create_embed.field("Max Cpu Freq", fmt_cpu_frequency(max_cpu_frequency), true);
        }
    }

    match (cpu_logical_count, cpu_physical_count) {
        (Some(logical_count), Some(physical_count)) => {
            create_embed = create_embed.field(
                "Cpu Core Count",
                format!("{logical_count} logical, {physical_count} physical"),
                true,
            );
        }
        (Some(logical_count), None) => {
            create_embed =
                create_embed.field("Cpu Core Count", format!("{logical_count} logical"), true);
        }
        (None, Some(physical_count)) => {
            create_embed =
                create_embed.field("Cpu Core Count", format!("{physical_count} physical"), true);
        }
        (None, None) => {}
    }

    if let (Some(total_memory), Some(available_memory)) = (total_memory, available_memory) {
        create_embed = create_embed.field(
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
        create_embed = create_embed.field(
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
    create_embed = create_embed.field("Virtualization", virtualization_field, true);

    if let (Some(cpu_usage), Some(cpu_logical_count)) = (cpu_usage, cpu_logical_count) {
        create_embed = create_embed.field(
            "Cpu Usage",
            format!("{:.2}%", cpu_usage / (cpu_logical_count as f32)),
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
        create_embed = create_embed.field("Cpu Temp", format!("{cpu_temp} °C"), true);
    }

    let footer = CreateEmbedFooter::new(format!(
        "Retrieved system data in {:.2} second(s)",
        data_retrieval_time.as_secs_f32()
    ));
    create_embed = create_embed.footer(footer);

    let create_reply = CreateReply::new().embed(create_embed);
    ctx.send(create_reply).await?;

    Ok(())
}
