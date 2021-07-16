#[derive(argh::FromArgs)]
#[argh(subcommand, name = "search", description = "search for a rule34 post")]
pub struct Options {
    #[argh(positional, description = "the query string")]
    query: String,

    #[argh(
        option,
        long = "offset",
        default = "0",
        description = "the starting offset"
    )]
    offset: u64,
}

pub async fn exec(client: &rule34::Client, options: Options) -> anyhow::Result<()> {
    let results = client.search(&options.query, options.offset).await?;

    if results.entries.is_empty() {
        println!("No Results");
    }

    for (i, result) in results.entries.iter().enumerate() {
        println!("{})", i + 1);
        println!("ID: {}", result.id);
        println!("Url: {}", result.get_post_url());
        println!("Description: {}", result.description);
        println!();
    }

    Ok(())
}
