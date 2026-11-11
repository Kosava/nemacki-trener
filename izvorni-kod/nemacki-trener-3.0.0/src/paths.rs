//! Gde se čuvaju baza i korisnikove liste reči.
//!
//! * "prenosivi" režim: ako pored programa već postoji nemacki_recnik.db,
//!   koristi se taj fajl (kao u Python verziji);
//! * inače: ~/.local/share/nemacki-trener (Linux), %APPDATA%\NemackiTrener (Windows).

use std::path::PathBuf;

use crate::db::DB_FILENAME;

pub struct Paths {
    pub data_dir: PathBuf,
    pub db: PathBuf,
    /// korisnikovi .txt fajlovi sa dodatnim rečima
    pub user_lists: PathBuf,
}

impl Paths {
    pub fn resolve() -> Self {
        let data_dir = if let Ok(dir) = std::env::var("NEMACKI_TRENER_DIR") {
            PathBuf::from(dir)
        } else if let Some(dir) = portable_dir() {
            dir
        } else {
            let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
            base.join(if cfg!(windows) { "NemackiTrener" } else { "nemacki-trener" })
        };
        Self {
            db: data_dir.join(DB_FILENAME),
            user_lists: data_dir.join("liste"),
            data_dir,
        }
    }
}

fn portable_dir() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    dir.join(DB_FILENAME).is_file().then(|| dir.to_path_buf())
}
