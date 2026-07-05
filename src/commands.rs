pub mod cmd;
pub mod deviantart;
pub mod help;
pub mod insta_dl;
pub mod iqdb;
pub mod latency;
pub mod leave;
pub mod nekos;
pub mod ping;
pub mod quizizz;
pub mod r6tracker;
pub mod reddit;
pub mod reddit_embed;
pub mod rule34;
pub mod sauce_nao;
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
    cmd::CMD_COMMAND,
    deviantart::DEVIANTART_COMMAND,
    help::help,
    insta_dl::INSTA_DL_COMMAND,
    iqdb::IQDB_COMMAND,
    latency::LATENCY_COMMAND,
    leave::LEAVE_COMMAND,
    nekos::nekos,
    ping::ping,
    quizizz::quizizz,
    r6tracker::r6tracker,
    reddit::reddit,
    reddit_embed::REDDIT_EMBED_COMMAND,
    rule34::rule34,
    sauce_nao::SAUCE_NAO_COMMAND,
    stop::STOP_COMMAND,
    system::SYSTEM_COMMAND,
    tic_tac_toe::TIC_TAC_TOE_COMMAND,
    tiktok_embed::tiktok_embed,
    urban::urban,
    uwuify::uwuify,
    vaporwave::vaporwave,
    xkcd::xkcd,
    yodaspeak::yodaspeak,
    zalgo::zalgo,
};
