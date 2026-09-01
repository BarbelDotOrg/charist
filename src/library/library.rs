use crate::bibles::{BibleData, load_bible_from_bytes};
use crate::consts::DEFAULT_BIBLE_NAME;
use std::path::PathBuf;
use std::{fs, io};

pub fn bibles_dir() -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("org.barbel.Charist").join("bibles"))
}

pub fn ensure_bibles_dir() -> io::Result<PathBuf> {
    let dir = bibles_dir().ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "no data dir"))?;
    fs::create_dir_all(&dir)?;
    Ok(dir)
}

#[derive(Debug, Clone)]
pub struct InstalledBible {
    pub name: String, // stem of the file, matches IndexedBible::name
    pub path: PathBuf,
}

pub fn list_installed() -> Vec<InstalledBible> {
    let Some(dir) = bibles_dir() else {
        return vec![];
    };
    let Ok(entries) = fs::read_dir(&dir) else {
        return vec![];
    };
    entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "cbor"))
        .filter_map(|e| {
            let path = e.path();
            let name = path.file_stem()?.to_string_lossy().to_string();
            Some(InstalledBible { name, path })
        })
        .collect()
}

pub fn bible_path(name: &str) -> Option<PathBuf> {
    bibles_dir().map(|d| d.join(format!("{name}.cbor")))
}

pub fn is_installed(name: &str) -> bool {
    bible_path(name).is_some_and(|p| p.exists())
}

pub fn delete_installed(name: &str) -> io::Result<()> {
    if let Some(path) = bible_path(name) {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

pub fn load_bible_from_disk(name: &str) -> Option<BibleData> {
    let path = bible_path(name)?;
    let bytes = fs::read(path).ok()?;
    load_bible_from_bytes(&bytes).ok()
}

/// Downloads and writes to disk. Blocking fs calls are fine here — files are small
/// and this already runs off the UI thread via `Task::perform`.
pub async fn download_bible(name: String, url: String) -> Result<(), String> {
    let bytes = reqwest::get(&url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    let dir = ensure_bibles_dir().map_err(|e| e.to_string())?;
    let path = dir.join(format!("{name}.cbor"));
    fs::write(path, bytes).map_err(|e| e.to_string())
}

pub fn ensure_default_bible_installed() -> io::Result<()> {
    if !list_installed().is_empty() {
        return Ok(());
    }
    let dir = ensure_bibles_dir()?;
    let path = dir.join(format!("{DEFAULT_BIBLE_NAME}.cbor"));
    fs::write(path, crate::assets::KJV)?;
    Ok(())
}
