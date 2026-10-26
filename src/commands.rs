pub mod deviantart;
pub mod iqdb;
pub mod latency;
pub mod leave;
pub mod nekos;
pub mod ping;
pub mod quizizz;
pub mod r6tracker;
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
    deviantart::deviantart,
    iqdb::iqdb,
    latency::LATENCY_COMMAND,
    leave::leave,
    nekos::nekos,
    ping::ping,
    quizizz::quizizz,
    r6tracker::r6tracker,
    rule34::rule34,
    sauce_nao::sauce_nao,
    stop::stop,
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
