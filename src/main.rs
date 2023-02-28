use fs4::tokio::AsyncFileExt;
use std::convert::identity;

use tokio::{
    fs::{self, DirEntry},
    process, time,
};
use tokio_stream::{Stream, StreamExt};

mod activity;
mod pausable_process;
mod settings;

use pausable_process::PausableProcess;
use settings::Settings;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let settings = Settings::init().expect("Failed to load settings");
    let path = &settings.videos_folder;

    loop {
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

        let videos = Box::pin(videos);
        run_encode(videos, &settings).await;
        time::sleep(time::Duration::from_secs(1)).await;
    }
}

/// doesn't actually hold exclusive lock.
async fn wait_until_exclusive(file: fs::File) -> Result<fs::File, std::io::Error> {
    tokio::task::spawn_blocking(move || {
        file.lock_exclusive()?;
        file.unlock()?;
        Ok(file)
    })
    .await
    .unwrap()
}

/// doesn't actually hold exclusive lock.
fn try_exclusive(file: fs::File) -> Result<bool, std::io::Error> {
    match file.try_lock_exclusive() {
        Ok(_) => file.unlock()?,
        Err(_) => return Ok(false),
    }
    Ok(true)
}

async fn run_encode<T: Stream<Item = DirEntry> + Unpin>(mut videos: T, settings: &Settings) {
    while let Some(file) = videos.next().await {
        println!("{}", file.file_name().to_str().unwrap());
        let mut output_path = settings.output_folder.join(file.file_name());
        if !settings.output_file_extension.is_empty() {
            output_path.set_extension(&settings.output_file_extension);
        }

        let f = fs::File::open(file.path()).await.unwrap();
        // if we aren't exclusive just try the next file, this one is actively being used for something.
        // e.g. recording.
        if mime_guess::from_path(file.path())
            .first_or_octet_stream()
            .type_()
            == mime::VIDEO
        {
            continue;
        }
        if !try_exclusive(f).unwrap() {
            continue;
        }

        let child = process::Command::new("ffmpeg.exe")
            .arg("-y")
            .arg("-xerror")
            .arg("-nostdin")
            .arg("-i")
            .arg(&file.path())
            .args(shell_words::split(&settings.ffmpeg_flags).expect("failed to parse ffmpeg flags"))
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
                    wait_until_active(time::Duration::from_secs(settings.wait_time)).await;
                    proc.unpause().unwrap();
                }
                status = proc.wait() => {
                    match status {
                        // While our file removal stuff is going we can start on the next batch.
                        Ok(s) if s.success() => tokio::task::spawn(async move {
                            let f = tokio::fs::File::open(file.path()).await.unwrap();

                            // wait for us to be able to get an exclusive lock.
                            // note: we immediately unlock and then try to delete it.
                            // It would be ideal if we could lock and delete so we know no other
                            // process is using it, but whatever. We do it this way just to attempt
                            // to make sure no one else is using the file for whatever reason.
                            let _ = wait_until_exclusive(f).await.unwrap();

                            // *try* to delete the file.
                            // ignore errors
                            let _ = fs::remove_file(file.path()).await;
                        }).await.unwrap(),
                        _ => ()
                    }
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
