#[derive(argh::FromArgs)]
#[argh(description = "a tool to download posts from instagram")]
struct CommandOptions {
    /// the url
    #[argh(positional)]
    url: String,
}

fn main() {
    let options: CommandOptions = argh::from_env();
    let tokio_rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(tokio_rt) => tokio_rt,
        Err(e) => {
            eprintln!("Failed to start tokio runtime: {}", e);
            return;
        }
    };

    tokio_rt.block_on(async_main(options));
}

async fn async_main(options: CommandOptions) {
    let client = insta::Client::new();

    let post = match client.get_post(&options.url).await {
        Ok(post) => post,
        Err(e) => {
            eprintln!("Failed to get post: {}", e);
            return;
        }
    };

    dbg!(post);
}
