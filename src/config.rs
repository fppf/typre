use std::{
    collections::HashMap,
    env, fmt, fs, io,
    path::{Path, PathBuf},
};

use crate::theme::{Theme, ThemeError, Themes};

#[derive(Debug)]
pub struct Config {
    pub db_path: PathBuf,
    pub sets: HashMap<String, PathBuf>,
    pub theme: Theme,
    pub themes: Themes,
    pub show_bg: bool,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let value = fs::read_to_string(path)
            .map_err(|e| ConfigError::Read(e.to_string()))?
            .parse::<toml::Value>()
            .map_err(|e| ConfigError::Toml(e.to_string()))?;

        let db_path = match value.get("db").and_then(|v| v.as_str()) {
            Some(path) => path.into(),
            None => default_data_dir()
                .map(|dir| dir.join("typre.db"))
                .ok_or(ConfigError::NoDatabase)?,
        };

        let sets_dir = match value.get("sets_dir").and_then(|v| v.as_str()) {
            Some(path) => path.into(),
            None => default_data_dir()
                .map(|dir| dir.join("sets"))
                .ok_or(ConfigError::NoSetsDir)?,
        };

        if !sets_dir.exists() || !sets_dir.is_dir() {
            return Err(ConfigError::InvalidSetsDir(sets_dir));
        }

        let sets =
            collect_word_sets(&sets_dir).map_err(|e| ConfigError::CollectSets(e.to_string()))?;

        let themes = Themes::from_value(&value).map_err(ConfigError::Theme)?;
        let theme_name = value.get("theme").and_then(|v| v.as_str());
        let theme = match theme_name {
            Some(name) => themes
                .get(name)
                .ok_or_else(|| ConfigError::NoTheme(name.into()))?,
            None => Theme::default(),
        };

        let show_bg = value
            .get("show_bg")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        Ok(Self {
            db_path,
            sets,
            theme,
            themes,
            show_bg,
        })
    }
}

fn collect_word_sets(sets_dir: &Path) -> io::Result<HashMap<String, PathBuf>> {
    let mut sets = HashMap::new();

    for entry in sets_dir.read_dir()? {
        let path = entry?.path();
        let path = path.canonicalize()?;

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("Fatal: word set stem is invalid UTF-8");
        sets.insert(stem.to_string(), path);
    }

    Ok(sets)
}

#[derive(Debug)]
pub enum ConfigError {
    Read(String),
    Toml(String),
    Theme(ThemeError),
    NoTheme(String),
    NoDatabase,
    NoSetsDir,
    InvalidSetsDir(PathBuf),
    CollectSets(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(e) => write!(f, "Failed to read configuration: {}", e),
            Self::Toml(e) => write!(f, "TOML parse error in configuration: {}", e),
            Self::Theme(e) => write!(f, "Theme set error: {:?}", e),
            Self::NoTheme(theme) => write!(f, "No theme '{}' found", theme),
            Self::NoDatabase => write!(f, "Database not specified"),
            Self::NoSetsDir => write!(f, "No word set directory specified"),
            Self::InvalidSetsDir(path) => {
                write!(f, "Invalid word set directory '{}'", path.display())
            }
            Self::CollectSets(e) => write!(f, "Failed to read sets: {}", e),
        }
    }
}

pub fn default_config_dir() -> Option<PathBuf> {
    xdg_config().map(|dir| dir.join("typre"))
}

pub fn default_data_dir() -> Option<PathBuf> {
    xdg_data().map(|dir| dir.join("typre"))
}

#[cfg(unix)]
fn xdg_config() -> Option<PathBuf> {
    Some(match env::var("XDG_CONFIG_HOME") {
        Ok(xdg_config) => PathBuf::from(xdg_config),
        Err(_) => {
            let home = env::var("HOME").ok()?;
            PathBuf::from(home).join(".config")
        }
    })
}

#[cfg(unix)]
fn xdg_data() -> Option<PathBuf> {
    Some(match env::var("XDG_DATA_HOME") {
        Ok(xdg_data) => PathBuf::from(xdg_data),
        Err(_) => {
            let home = env::var("HOME").ok()?;
            PathBuf::from(home).join(".local/share")
        }
    })
}
