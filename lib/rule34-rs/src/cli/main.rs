mod commands;

use anyhow::Context;

#[derive(argh::FromArgs)]
#[argh(description = "A utility to get rule34 images")]
pub struct Options {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum SubCommand {
    Search(self::commands::search::Options),
    Download(self::commands::download::Options),
    Deleted(self::commands::deleted::Options),
}

fn main() {
    let options: Options = argh::from_env();
    let exit_code = {
        if let Err(e) = real_main(options) {
            eprintln!("{:?}", e);
            1
        } else {
            0
        }
    };

    std::process::exit(exit_code);
}

fn real_main(options: Options) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start tokio runtime")?;
    tokio_rt.block_on(async_main(options))?;
    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = rule34::Client::new();

    match options.subcommand {
        SubCommand::Search(options) => self::commands::search::exec(&client, options).await?,
        SubCommand::Download(options) => self::commands::download::exec(&client, options).await?,
        SubCommand::Deleted(options) => self::commands::deleted::exec(&client, options).await?,
    }

    Ok(())
}
