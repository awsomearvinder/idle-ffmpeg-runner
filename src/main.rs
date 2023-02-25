use std::convert::identity;

use tokio_stream::StreamExt;

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
        let mut pause_on_drop = pausable_process::PauseOnDrop::new(&mut proc);

        tokio::select! {
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(10)) => {
                  proc.pause().unwrap();
                println!("time finished first");
            }
            status = pause_on_drop.wait() => {
                println!("finished! {}", status.unwrap());
            },
        }
    }
    println!("Hello, world!");
}
