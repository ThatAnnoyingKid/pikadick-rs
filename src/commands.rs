pub mod cache_stats;
pub mod chat;
pub mod cmd;
pub mod deviantart;
pub mod fml;
pub mod help;
pub mod insta_dl;
pub mod invite;
pub mod iqdb;
pub mod latency;
pub mod leave;
pub mod nekos;
pub mod ping;
pub mod quizizz;
pub mod r6stats;
pub mod r6tracker;
pub mod reddit;
pub mod reddit_embed;
pub mod rule34;
pub mod sauce_nao;
pub mod shift;
pub mod stop;
pub mod system;
pub mod tic_tac_toe;
pub mod tiktok_embed;
pub mod urban;
pub mod uwuify;
pub mod vaporwave;
pub mod xkcd;
pub mod yodaspeak;
pub mod zalgo;

pub use self::{
    cache_stats::CACHE_STATS_COMMAND,
    cmd::CMD_COMMAND,
    deviantart::DEVIANTART_COMMAND,
    fml::FML_COMMAND,
    help::help,
    insta_dl::INSTA_DL_COMMAND,
    invite::INVITE_COMMAND,
    iqdb::IQDB_COMMAND,
    latency::LATENCY_COMMAND,
    leave::LEAVE_COMMAND,
    nekos::nekos,
    ping::ping,
    quizizz::QUIZIZZ_COMMAND,
    r6stats::r6stats,
    reddit::REDDIT_COMMAND,
    reddit_embed::REDDIT_EMBED_COMMAND,
    sauce_nao::SAUCE_NAO_COMMAND,
    shift::SHIFT_COMMAND,
    stop::STOP_COMMAND,
    system::SYSTEM_COMMAND,
    tic_tac_toe::TIC_TAC_TOE_COMMAND,
    urban::URBAN_COMMAND,
    uwuify::UWUIFY_COMMAND,
    vaporwave::VAPORWAVE_COMMAND,
    xkcd::XKCD_COMMAND,
    zalgo::ZALGO_COMMAND,
};
