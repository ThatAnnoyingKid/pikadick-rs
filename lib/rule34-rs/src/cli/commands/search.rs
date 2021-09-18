use std::str::FromStr;

#[derive(argh::FromArgs)]
#[argh(subcommand, name = "search", description = "search for a rule34 post")]
pub struct Options {
    #[argh(positional, description = "the query string")]
    query: String,

    #[argh(
        option,
        long = "offset",
        short = 'o',
        default = "0",
        description = "the starting offset"
    )]
    offset: u64,

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
    let results = client.search(&options.query, options.offset).await?;

    match options.output_type {
        OutputType::Human => {
            if results.is_empty() {
                println!("No Results");
            }

            for (i, result) in results.iter().enumerate() {
                println!("{})", i + 1);
                println!("ID: {}", result.id);
                println!("Url: {}", result.get_post_url());
                println!("Tags: {}", result.tags);
                println!();
            }
        }
        OutputType::Json => {
            println!("{}", serde_json::to_string(&results)?);
        }
    }

    Ok(())
}
