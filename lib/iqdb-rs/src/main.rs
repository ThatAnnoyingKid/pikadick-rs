use anyhow::Context;

#[derive(argh::FromArgs)]
#[argh(description = "A tool to look up images on iqdb")]
struct Options {
    #[argh(positional, description = "the target url")]
    url: String,
}

fn main() {
    let options: Options = argh::from_env();
    let code = match real_main(options) {
        Ok(()) => 0,
        Err(e) => {
            eprintln!("{:?}", e);
            1
        }
    };

    std::process::exit(code);
}

fn real_main(options: Options) -> anyhow::Result<()> {
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to start tokio runtime")?;

    tokio_rt.block_on(async_main(options))?;
    eprintln!("Done.");

    Ok(())
}

async fn async_main(options: Options) -> anyhow::Result<()> {
    let client = iqdb::Client::new();

    let image: iqdb::Image = if options.url.starts_with("http") {
        options.url.as_str().into()
    } else {
        eprintln!("Opening image...");
        iqdb::Image::from_path(options.url.as_ref())
            .await
            .context("failed to load image")?
    };

    eprintln!("Searching...");
    eprintln!();
    let search_results = client.search(image).await.context("failed to search")?;

    if let Some(best_match) = search_results.best_match.as_ref() {
        println!("Best Match");
        println!(" * Url: {}", best_match.url.as_str());
        println!(" * Image Url: {}", best_match.image_url.as_str());
    } else {
        println!("No Best Match");
    }

    println!();
    println!();

    for (i, possible_match) in search_results.possible_matches.iter().enumerate() {
        println!("Possbile Match {}", i + 1);
        println!(" * Url: {}", possible_match.url.as_str());
        println!(" * Image Url: {}", possible_match.image_url.as_str());
        println!();
    }

    Ok(())
}
