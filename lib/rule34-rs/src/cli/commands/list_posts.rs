use std::str::FromStr;

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "list-posts", description = "list rule34 posts")]
pub struct Options {
    #[argh(option, long = "tags", description = "the tags")]
    tags: Option<String>,

    #[argh(option, long = "pid", short = 'p', description = "the page #")]
    pid: Option<u64>,

    #[argh(option, long = "id", description = "the post id")]
    id: Option<u64>,

    #[argh(
        option,
        long = "limit",
        short = 'l',
        description = "the # of posts per page"
    )]
    limit: Option<u16>,

    #[argh(
        option,
        long = "output-type",
        short = 't',
        default = "OutputType::Human",
        description = "the output type"
    )]
    output_type: OutputType,
}

#[derive(Debug)]
pub struct OutputTypeParseError(String);

impl std::fmt::Display for OutputTypeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "'{}' is not valid. Try 'human' or 'json'.", self.0)
    }
}

/// The output type
#[derive(Debug, Clone, Copy)]
pub enum OutputType {
    Human,
    Json,
}

impl FromStr for OutputType {
    type Err = OutputTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "h" | "human" => Ok(Self::Human),
            "j" | "json" => Ok(Self::Json),
            s => Err(OutputTypeParseError(s.into())),
        }
    }
}

pub async fn exec(client: &rule34::Client, options: Options) -> anyhow::Result<()> {
    let list = client
        .list_posts()
        .tags(options.tags.as_deref())
        .pid(options.pid)
        .id(options.id)
        .limit(options.limit)
        .execute()
        .await?;

    match options.output_type {
        OutputType::Human => {
            println!("Count: {}", list.count);
            println!("Offset: {}", list.offset);
            if list.posts.is_empty() {
                println!("No Results");
            }

            for (i, post) in list.posts.iter().enumerate() {
                println!("{})", i + 1);
                println!("ID: {}", post.id);
                println!("Url: {}", post.get_html_post_url());
                println!("Tags: {}", post.tags);
                println!();
            }
        }
        OutputType::Json => {
            println!("{}", serde_json::to_string(&list)?);
        }
    }

    Ok(())
}
