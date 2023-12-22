mod commands;

#[derive(argh::FromArgs)]
#[argh(description = "A CLI to interact with rule34.xxx")]
pub struct Options {
    #[argh(subcommand)]
    subcommand: SubCommand,
}

#[derive(argh::FromArgs)]
#[argh(subcommand)]
enum SubCommand {
    ListPosts(self::commands::list_posts::Options),
    Download(self::commands::download::Options),
    Deleted(self::commands::deleted::Options),
}

fn main() -> anyhow::Result<()> {
    let options: Options = argh::from_env();
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    tokio_rt.block_on(async_main(options))?;
    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = rule34::Client::new();

    match options.subcommand {
        SubCommand::ListPosts(options) => {
            self::commands::list_posts::exec(&client, options).await?
        }
        SubCommand::Download(options) => self::commands::download::exec(&client, options).await?,
        SubCommand::Deleted(options) => self::commands::deleted::exec(&client, options).await?,
    }

    Ok(())
}
