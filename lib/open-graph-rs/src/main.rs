use std::path::Path;

#[derive(argh::FromArgs)]
#[argh(description = "a tool to download media from open-graph compatible sources")]
struct CommandOptions {
    /// the url to a ogp compatible webpage
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
    println!("Done");
}

async fn async_main(options: CommandOptions) {
    let client = open_graph::Client::new();

    let object = match client.get_object(&options.url).await {
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

        let extension = match Path::new(video_url.path())
            .extension()
            .map(|extension| extension.to_str())
        {
            Some(Some(extension)) => extension,
            Some(None) => {
                eprintln!("Invalid extension, using 'mp4'");
                "mp4"
            }
            None => {
                eprintln!("Unknown extension, using 'mp4'");
                "mp4"
            }
        };

        let filename = format!("video.{}", extension);

        download(&client, video_url.as_str(), &filename).await;

        return;
    }

    if object.kind == "instapp:photo" {
        let extension = match Path::new(object.image.path())
            .extension()
            .map(|extension| extension.to_str())
        {
            Some(Some(extension)) => extension,
            Some(None) => {
                eprintln!("Invalid extension, using 'png'");
                "png"
            }
            None => {
                eprintln!("Unknown extension, using 'png'");
                "png"
            }
        };

        let filename = format!("image.{}", extension);
        download(&client, object.image.as_str(), &filename).await;
        return;
    }

    eprintln!("Unsupported object type");
    dbg!(object);

    return;
}

/// Download a url's contents
async fn download(client: &open_graph::Client, url: &str, filename: &str) {
    let response = match client.client.get(url).send().await {
        Ok(res) => res,
        Err(e) => {
            eprintln!("Failed to send request: {}", e);
            return;
        }
    };
    let status = response.status();
    if !status.is_success() {
        eprintln!("Invalid HTTP Status Code {}", status);
        return;
    }

    let data = match response.bytes().await {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Failed to download request body: {}", e);
            return;
        }
    };

    if let Err(e) = std::fs::write(filename, data) {
        eprintln!("Failed to save file '{}': {}", filename, e);
    }
}
