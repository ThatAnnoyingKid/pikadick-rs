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

    let object = match client.get_post(&options.url).await {
        Ok(object) => object,
        Err(e) => {
            eprintln!("Failed to get post: {}", e);
            return;
        }
    };

    if object.is_video() {
        let video_url = match object.video_url.as_ref() {
            Some(url) => url,
            None => {
                eprintln!("Missing video url");
                return;
            }
        };

        let res = match client.client.get(video_url.as_str()).send().await {
            Ok(res) => res,
            Err(e) => {
                eprintln!("Failed to send request: {}", e);
                return;
            }
        };
        let status = res.status();
        if !status.is_success() {
            eprintln!("Invalid HTTP Status Code {}", status);
            return;
        }

        let data = match res.bytes().await {
            Ok(data) => data,
            Err(e) => {
                eprintln!("Failed to download request body: {}", e);
                return;
            }
        };

        if let Err(e) = std::fs::write("video.mp4", data) {
            eprintln!("Failed to save video: {}", e);
        }
    } else {
        eprintln!("Unsupported object type");
        dbg!(object);
    }
}
