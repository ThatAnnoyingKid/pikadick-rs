pub mod cache_stats;
pub mod cmd;
pub mod deviantart;
pub mod fml;
pub mod insta_dl;
pub mod invite;
pub mod latency;
pub mod nekos;
pub mod ping;
pub mod polldaddy;
pub mod quizizz;
pub mod r6stats;
pub mod r6tracker;
pub mod reddit_embed;
pub mod rule34;
pub mod shift;
pub mod system;
pub mod uwuify;
pub mod vaporwave;
pub mod zalgo;

pub use crate::commands::{
    cache_stats::CACHE_STATS_COMMAND,
    cmd::CMD_COMMAND,
    deviantart::DEVIANTART_COMMAND,
    fml::FML_COMMAND,
    insta_dl::INSTA_DL_COMMAND,
    invite::INVITE_COMMAND,
    latency::LATENCY_COMMAND,
    nekos::NEKOS_COMMAND,
    ping::PING_COMMAND,
    polldaddy::POLLDADDY_COMMAND,
    quizizz::QUIZIZZ_COMMAND,
    r6stats::R6STATS_COMMAND,
    r6tracker::R6TRACKER_COMMAND,
    reddit_embed::REDDIT_EMBED_COMMAND,
    rule34::RULE34_COMMAND,
    shift::SHIFT_COMMAND,
    system::SYSTEM_COMMAND,
    uwuify::UWUIFY_COMMAND,
    vaporwave::VAPORWAVE_COMMAND,
    zalgo::ZALGO_COMMAND,
};
