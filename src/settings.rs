use std::path::PathBuf;

use config::{Config, ConfigError, Environment, File};
use directories::UserDirs;

#[derive(serde::Deserialize, PartialEq, Eq, Hash, Debug)]
pub struct Settings {
    #[serde(default = "video_folder")]
    pub videos_folder: PathBuf,
    #[serde(default = "encoded_folder")]
    pub output_folder: PathBuf,
}

fn video_folder() -> PathBuf {
    let dirs = UserDirs::new().unwrap();
    dirs.video_dir().unwrap().to_owned()
}

fn encoded_folder() -> PathBuf {
    let dirs = UserDirs::new().unwrap();
    dirs.video_dir().unwrap().to_owned().join("encoded")
}

impl Settings {
    pub fn init() -> Result<Self, ConfigError> {
        Config::builder()
            .add_source(File::with_name("default.toml"))
            .add_source(File::with_name("ffmpeg_idle.toml").required(false))
            .add_source(Environment::with_prefix("FFMPEG_IDLE"))
            .build()?
            .try_deserialize()
    }
}
