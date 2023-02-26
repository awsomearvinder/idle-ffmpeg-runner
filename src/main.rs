use std::convert::identity;

use std::{env, path};

use tokio::{fs, process, time};
use tokio_stream::StreamExt;

mod activity;
mod pausable_process;

use pausable_process::PausableProcess;

#[tokio::main]
async fn main() {
    let folder = env::var_os("USERPROFILE").unwrap();
    let path = path::PathBuf::from(folder).join(r"Videos");
    let entries = fs::read_dir(path).await.unwrap();
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
        let child = process::Command::new("ffmpeg.exe")
            .arg("-y")
            .arg("-i")
            .arg(&file.path())
            .arg("output.mp4")
            .spawn()
            .unwrap();

        let mut proc = PausableProcess::new(child);
        loop {
            // race user input and ffmpeg
            // if user input finishes first, pause ffmpeg and wait for the user to be active.
            tokio::select! {
                _ = activity::get_input() => {
                    proc.pause().unwrap();
                    wait_until_active(time::Duration::from_secs(60 * 60)).await;
                    proc.unpause().unwrap();
                }
                status = proc.wait() => {
                    println!("finished! {}", status.unwrap());
                    break;
                },
            }
        }
    }
}

async fn wait_until_active(duration_without_activity: time::Duration) {
    loop {
        tokio::select! {
            _ = activity::get_input() => {
                continue;
            }

            _ = time::sleep(duration_without_activity) => {
                break;
            }
        }
    }
}
