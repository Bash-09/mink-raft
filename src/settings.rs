use std::path::{Path, PathBuf};

use directories_next::ProjectDirs;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub direct_connection: String,
    pub show_fps: bool,
    pub vsync: bool,

    pub window_pos: Option<[i32; 2]>,
    pub window_size: [u32; 2],

    pub mouse_sensitivity: f64,
    pub fov: f64,

    pub online_play: bool,
    pub name: String,
    pub saved_servers: Vec<SavedServer>,

    pub day_colour: [f32; 3],
    pub fog_near: f32,
    pub fog_far: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Hash, Default)]
#[serde(default)]
pub struct SavedServer {
    pub ip: String,
    pub name: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Ser/De error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("No valid home directory found")]
    NoValidHome,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            direct_connection: String::new(),
            show_fps: true,
            vsync: true,

            window_pos: None,
            window_size: [1200, 700],

            mouse_sensitivity: 1.0,
            fov: 90.0,

            online_play: false,

            name: String::from("Bash"),
            saved_servers: Vec::new(),

            day_colour: [0.3, 0.6, 0.9],
            fog_near: 5.0,
            fog_far: 320.0,
        }
    }
}

impl Settings {
    pub fn load_from<P: AsRef<Path>>(file: P) -> Result<Settings, Error> {
        let contents = std::fs::read_to_string(file)?;
        let settings = serde_yaml::from_str(&contents)?;

        Ok(settings)
    }

    pub fn save_to<P: AsRef<Path>>(&self, file: P) -> Result<(), Error> {
        let contents = serde_yaml::to_string(self)?;
        std::fs::write(file, contents)?;

        Ok(())
    }

    pub fn load() -> Result<Settings, Error> {
        let path = locate_config_directory()?.join("config.yaml");
        Self::load_from(path)
    }

    pub fn save(&self) -> Result<(), Error> {
        let path = locate_config_directory()?.join("config.yaml");
        self.save_to(path)
    }
}

pub fn locate_config_directory() -> Result<PathBuf, Error> {
    let dirs = ProjectDirs::from("mink-raft", "bash", "mink-raft").ok_or(Error::NoValidHome)?;
    let dir = dirs.config_dir();
    std::fs::create_dir_all(dir)?;
    Ok(dir.into())
}
