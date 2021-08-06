use std::path::PathBuf;
use url::Url;

#[derive(argh::FromArgs)]
#[argh(description = "App to download tiktok videos")]
struct CommandOptions {
    /// the post url
    #[argh(positional)]
    url: Url,

    /// the outfile
    #[argh(option, short = 'o', default = "PathBuf::from(\"video.mp4\")")]
    out_file: PathBuf,
}

fn main() {
    let options: CommandOptions = argh::from_env();

    let tokio_rt = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("Failed to start tokio runtime: {}", e);
            return;
        }
    };

    let client = tiktock::Client::new();

    tokio_rt.block_on(async {
        let url = match tiktock::PostUrl::from_url(options.url) {
            Ok(url) => url,
            Err(_e) => {
                eprintln!("Invalid post url");
                return;
            }
        };

        eprintln!("Fetching post page...");
        let post = match client.get_post(&url).await {
            Ok(post) => post,
            Err(e) => {
                eprintln!("Failed to get post: {}", e);
                return;
            }
        };

        let video_url = match post.video_url.as_ref() {
            Some(url) => url,
            None => {
                eprintln!("Missing video url");
                return;
            }
        };

        eprintln!("Downloading video from '{}'", video_url.as_str());
        let mut file = match tokio::fs::File::create(&options.out_file).await {
            Ok(file) => file,
            Err(e) => {
                eprintln!("Failed to create file: {}", e);
                return;
            }
        };

        if let Err(e) = client.get_to(video_url.as_str(), &mut file).await {
            eprintln!("Failed to download video: {}", e);
            return;
        }
    });

    println!("Done.");
}
