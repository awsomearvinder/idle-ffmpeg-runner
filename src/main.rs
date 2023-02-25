use std::convert::identity;

use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let folder = std::env::var_os("USERPROFILE").unwrap();
    let path = std::path::PathBuf::from(folder).join(r"Videos");
    let entries = tokio::fs::read_dir(path).await.unwrap();
    let videos = tokio_stream::wrappers::ReadDirStream::new(entries)
        .filter_map(Result::ok)
        .then(|i| async {
            match i.file_type().await {
                Ok(v) if v.is_file() || v.is_symlink() => Some(i),
                _ => None,
            }
        })
        .filter_map(identity);

    let mut videos = Box::pin(videos);

    while let Some(file) = videos.next().await {
        println!("{}", file.file_name().to_str().unwrap());
        let output = tokio::process::Command::new("ffmpeg.exe")
            .arg("-i")
            .arg(&file.path())
            .arg("output.mp4")
            .output()
            .await;

        match output {
            Ok(v) => {
                let output = String::from_utf8(v.stdout).unwrap();
                let error = String::from_utf8(v.stderr).unwrap();
                println!("{output}\n{error}")
            }
            Err(v) => println!("{}", v.to_string()),
        }
    }
    println!("Hello, world!");
}
