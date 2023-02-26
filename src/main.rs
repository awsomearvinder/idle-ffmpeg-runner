use std::convert::identity;

use tokio::{fs, process, time};
use tokio_stream::StreamExt;

mod activity;
mod pausable_process;
mod settings;

use pausable_process::PausableProcess;
use settings::Settings;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let settings = Settings::init().expect("Failed to load settings");
    let path = settings.videos_folder;
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
        let output_path = settings.output_folder.join(file.file_name());
        let child = process::Command::new("ffmpeg.exe")
            .arg("-y")
            .arg("-i")
            .arg(&file.path())
            .arg(&settings.output_folder.join(output_path))
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
