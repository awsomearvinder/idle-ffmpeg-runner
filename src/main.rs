use std::convert::identity;

use tokio_stream::StreamExt;

mod activity;
mod pausable_process;

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
        let child = tokio::process::Command::new("ffmpeg.exe")
            .arg("-y")
            .arg("-i")
            .arg(&file.path())
            .arg("output.mp4")
            .spawn()
            .unwrap();

        let mut proc = pausable_process::PausableProcess::new(child);
        loop {
            // race user input and ffmpeg
            // if user input finishes first, pause ffmpeg and wait for the user to be active.
            tokio::select! {
                _ = activity::get_input() => {
                    proc.pause().unwrap();
                    wait_until_active(tokio::time::Duration::from_secs(60 * 60)).await;
                    proc.unpause().unwrap();
                }
                status = proc.wait() => {
                    println!("finished! {}", status.unwrap());
                    break;
                },
            }
        }
    }
    println!("Hello, world!");
}

async fn wait_until_active(duration_without_activity: tokio::time::Duration) {
    loop {
        tokio::select! {
            _ = activity::get_input() => {
                continue;
            }

            _ = tokio::time::sleep(duration_without_activity) => {
                break;
            }
        }
    }
}