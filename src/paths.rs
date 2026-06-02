//! Platform-correct filesystem layout for em.

use directories::ProjectDirs;
use std::path::{Path, PathBuf};

/// Application directories derived from OS conventions.
#[derive(Debug, Clone)]
pub struct AppDirs {
    root: PathBuf,
}

impl AppDirs {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn profiles_dir(&self) -> PathBuf {
        self.root.join("profiles")
    }

    pub fn presets_dir(&self) -> PathBuf {
        self.root.join("presets")
    }
}

pub fn app_dirs() -> std::io::Result<AppDirs> {
    let proj = ProjectDirs::from("io", "xenvmanager", "em").ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "unable to determine project directories",
        )
    })?;

    // Prefer config_dir (roaming on Windows). We can switch to config_local_dir later.
    Ok(AppDirs {
        root: proj.config_dir().to_path_buf(),
    })
}
