//! SQLite baza — ista šema kao u Python verziji (nemacki_recnik.db), pa se
//! postojeća baza sa napretkom može samo prekopirati.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::model::{level_order, topic_order, Entry};
use crate::text::{casefold, fold, natural_key};
use crate::wordlists;

pub const DB_FILENAME: &str = "nemacki_recnik.db";
/// Verzija podataka koju ova aplikacija očekuje (Python v2.1+ = 3).
pub const DATA_VERSION: i64 = 3;

pub type Result<T> = rusqlite::Result<T>;

/// Zapisi iz ranijih verzija lista koji su ispravljeni (nemački, stari srpski
/// prevod) — brišu se iz postojećih baza, a napredak prelazi na ispravljen zapis.
const RETIRED: &[(&str, &str, &str)] = &[("besonders", "poseban", "besonderer")];

pub struct Database {
    pub conn: Connection,
}

#[derive(Debug, Default)]
pub struct SyncReport {
    pub added: usize,
    pub replaced: usize,
    pub errors: Vec<String>,
}

const COLS: &str = "id, category, type, sr_text, de_text, pronunciation, example, level, pos, forms";

fn row_to_entry(r: &rusqlite::Row) -> Result<Entry> {
    let s = |i: usize| -> Result<String> { Ok(r.get::<_, Option<String>>(i)?.unwrap_or_default()) };
    Ok(Entry {
        id: r.get(0)?,
        category: s(1)?,
        kind: s(2)?,
        sr: s(3)?,
        de: s(4)?,
        pronunciation: s(5)?,
        example: s(6)?,
        level: s(7)?,
        pos: s(8)?,
        forms: s(9)?,
    })
}

pub fn progress_key(sr: &str, de: &str) -> String {
    format!("{}|{}", casefold(sr), casefold(de))
}

impl Database {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.create_schema()?;
        Ok(db)
    }

    #[cfg(test)]
    pub fn in_memory() -> Result<Self> {
        let db = Self { conn: Connection::open_in_memory()? };
        db.create_schema()?;
        Ok(db)
    }

    fn create_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS vocabulary (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category TEXT NOT NULL,
                type TEXT NOT NULL,
                sr_text TEXT NOT NULL,
                de_text TEXT NOT NULL,
                pronunciation TEXT DEFAULT '',
                example TEXT DEFAULT ''
            );
            CREATE UNIQUE INDEX IF NOT EXISTS idx_unique_entry
                ON vocabulary (LOWER(sr_text), LOWER(de_text), LOWER(category));
            CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value TEXT);
            CREATE TABLE IF NOT EXISTS progress (
                key       TEXT PRIMARY KEY,
                box       INTEGER NOT NULL DEFAULT 0,
                due       TEXT    NOT NULL,
                correct   INTEGER NOT NULL DEFAULT 0,
                wrong     INTEGER NOT NULL DEFAULT 0,
                last_seen TEXT    NOT NULL DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS review_log (
                id      INTEGER PRIMARY KEY AUTOINCREMENT,
                day     TEXT    NOT NULL,
                ns      TEXT    NOT NULL DEFAULT '',
                key     TEXT    NOT NULL,
                correct INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_review_log_day ON review_log(day);",
        )?;
        let mut have = HashSet::new();
        {
            let mut st = self.conn.prepare("PRAGMA table_info(vocabulary)")?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                have.insert(r.get::<_, String>(1)?);
            }
        }
        for col in ["level", "pos", "forms"] {
            if !have.contains(col) {
                self.conn.execute_batch(&format!(
                    "ALTER TABLE vocabulary ADD COLUMN {col} TEXT NOT NULL DEFAULT ''"
                ))?;
            }
        }
        Ok(())
    }

    // ---------- meta ----------
    pub fn get_meta(&self, key: &str) -> Option<String> {
        self.conn
            .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
            .optional()
            .ok()
            .flatten()
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO meta(key, value) VALUES(?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    // ---------- čitanje ----------
    pub fn count(&self) -> i64 {
        self.conn
            .query_row("SELECT COUNT(*) FROM vocabulary", [], |r| r.get(0))
            .unwrap_or(0)
    }

    /// Svi zapisi, sortirani: nivo, tema, srpski tekst.
    pub fn load_all(&self) -> Result<Vec<Entry>> {
        let mut st = self.conn.prepare(&format!("SELECT {COLS} FROM vocabulary"))?;
        let mut v: Vec<Entry> = st.query_map([], row_to_entry)?.collect::<Result<_>>()?;
        v.sort_by_cached_key(|e| {
            (level_order(&e.level), topic_order(&e.category), natural_key(&e.category), fold(&e.sr))
        });
        Ok(v)
    }

    pub fn exists(&self, sr: &str, de: &str, category: &str, exclude_id: i64) -> Result<bool> {
        let key = (casefold(sr), casefold(de), casefold(category));
        let mut st = self.conn.prepare("SELECT id, sr_text, de_text, category FROM vocabulary")?;
        let mut rows = st.query([])?;
        while let Some(r) = rows.next()? {
            let id: i64 = r.get(0)?;
            let k = (
                casefold(&r.get::<_, String>(1)?),
                casefold(&r.get::<_, String>(2)?),
                casefold(&r.get::<_, String>(3)?),
            );
            if id != exclude_id && k == key {
                return Ok(true);
            }
        }
        Ok(false)
    }

    // ---------- upis ----------
    pub fn add(&self, e: &Entry) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO vocabulary (category, type, sr_text, de_text, pronunciation, example,
                                     level, pos, forms)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![e.category, e.kind, e.sr, e.de, e.pronunciation, e.example, e.level, e.pos, e.forms],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update(&self, e: &Entry) -> Result<()> {
        self.conn.execute(
            "UPDATE vocabulary SET category=?1, type=?2, sr_text=?3, de_text=?4, pronunciation=?5,
                    example=?6, level=?7, pos=?8, forms=?9 WHERE id=?10",
            params![e.category, e.kind, e.sr, e.de, e.pronunciation, e.example, e.level, e.pos,
                    e.forms, e.id],
        )?;
        Ok(())
    }

    pub fn delete(&self, id: i64) -> Result<()> {
        self.conn.execute("DELETE FROM vocabulary WHERE id = ?1", [id])?;
        Ok(())
    }

    /// Ubacuje sve u jednoj transakciji; vraća broj dodatih (duplikati se preskaču).
    pub fn add_many(&mut self, entries: &[Entry]) -> Result<usize> {
        let mut existing: HashSet<(String, String, String)> = HashSet::new();
        {
            let mut st = self.conn.prepare("SELECT sr_text, de_text, category FROM vocabulary")?;
            let mut rows = st.query([])?;
            while let Some(r) = rows.next()? {
                existing.insert((
                    casefold(&r.get::<_, String>(0)?),
                    casefold(&r.get::<_, String>(1)?),
                    casefold(&r.get::<_, String>(2)?),
                ));
            }
        }
        let tx = self.conn.transaction()?;
        let mut added = 0;
        {
            let mut st = tx.prepare(
                "INSERT OR IGNORE INTO vocabulary (category, type, sr_text, de_text, pronunciation,
                                                   example, level, pos, forms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            )?;
            for e in entries {
                let k = (casefold(&e.sr), casefold(&e.de), casefold(&e.category));
                if !existing.insert(k) {
                    continue;
                }
                added += st.execute(params![
                    e.category, e.kind, e.sr, e.de, e.pronunciation, e.example, e.level, e.pos, e.forms
                ])?;
            }
        }
        tx.commit()?;
        Ok(added)
    }

    /// Prvo pokretanje i dopuna rečnika kad se data/*.txt promeni.
    ///
    /// * prazna baza: starije reči (legacy.tsv) + liste po nivoima;
    /// * postojeća baza: dodaju se nove reči iz lista; starije reči koje sada
    ///   postoje i u listama zamenjuju se novim zapisom (napredak se prenosi).
    pub fn sync_wordlists(&mut self, user_dir: Option<&Path>) -> Result<SyncReport> {
        let (rows, errors, hash) = wordlists::load_all(user_dir);
        let mut report = SyncReport { errors, ..Default::default() };
        let hash_s = format!("rs:{hash:016x}");
        let fresh = self.count() == 0;
        if !fresh && self.get_meta("wordlists_hash").as_deref() == Some(&hash_s) {
            return Ok(report);
        }

        // kurirane reči po (nemački, je_rečenica) -> srpski prevod
        let mut curated: HashMap<(String, bool), &wordlists::WordRow> = HashMap::new();
        for w in &rows {
            curated.entry((w.de.clone(), w.is_sentence())).or_insert(w);
        }
        let legacy = wordlists::legacy_entries();

        if fresh {
            let keep: Vec<Entry> = legacy
                .into_iter()
                .filter(|e| !curated.contains_key(&(e.de.clone(), e.is_sentence())))
                .collect();
            self.add_many(&keep)?;
            self.set_meta("data_version", &DATA_VERSION.to_string())?;
        } else {
            // starije reči koje su sada pokrivene listama -> zameni novim zapisom
            let legacy_keys: HashSet<(String, String, String, String)> = legacy
                .iter()
                .map(|e| (e.level.clone(), e.category.clone(), e.sr.clone(), e.de.clone()))
                .collect();
            let mut doomed: Vec<(i64, String, String)> = Vec::new(); // id, stari ključ, novi ključ
            {
                let mut st = self.conn.prepare(&format!("SELECT {COLS} FROM vocabulary"))?;
                let all: Vec<Entry> = st.query_map([], row_to_entry)?.collect::<Result<_>>()?;
                for e in all {
                    let Some(w) = curated.get(&(e.de.clone(), e.is_sentence())) else { continue };
                    let is_legacy = e.level == "X"
                        || legacy_keys.contains(&(
                            e.level.clone(), e.category.clone(), e.sr.clone(), e.de.clone(),
                        ));
                    let is_curated_row = e.level == w.level && e.category == w.topic && e.sr == w.sr;
                    if is_legacy && !is_curated_row {
                        doomed.push((e.id, progress_key(&e.sr, &e.de), progress_key(&w.sr, &w.de)));
                    }
                }
            }
            for (de, sr, new_de) in RETIRED {
                let id: Option<i64> = self
                    .conn
                    .query_row(
                        "SELECT id FROM vocabulary WHERE de_text = ?1 AND sr_text = ?2",
                        params![de, sr],
                        |r| r.get(0),
                    )
                    .optional()?;
                let new_sr = rows.iter().find(|w| w.de == *new_de).map(|w| w.sr.clone());
                if let (Some(id), Some(new_sr)) = (id, new_sr) {
                    doomed.push((id, progress_key(sr, de), progress_key(&new_sr, new_de)));
                }
            }
            let tx = self.conn.transaction()?;
            for (id, old_key, new_key) in &doomed {
                if old_key != new_key {
                    for ns in crate::progress::ALL_PREFIXES {
                        tx.execute(
                            "UPDATE OR IGNORE progress SET key = ?1 WHERE key = ?2",
                            params![format!("{ns}{new_key}"), format!("{ns}{old_key}")],
                        )?;
                    }
                }
                tx.execute("DELETE FROM vocabulary WHERE id = ?1", [id])?;
            }
            tx.commit()?;
            report.replaced = doomed.len();
        }

        let entries: Vec<Entry> = rows.iter().map(|w| w.to_entry()).collect();
        report.added = self.add_many(&entries)?;
        self.set_meta("wordlists_hash", &hash_s)?;
        Ok(report)
    }

    /// Verzija podataka u bazi (0 = nepoznato / vrlo stara baza).
    pub fn data_version(&self) -> i64 {
        self.get_meta("data_version").and_then(|v| v.parse().ok()).unwrap_or(0)
    }
}

/// {nivo: [(tema, broj zapisa)]} za dati skup zapisa, u redosledu prikaza.
pub fn level_topics<'a>(entries: impl Iterator<Item = &'a Entry>) -> Vec<(String, Vec<(String, usize)>)> {
    let mut map: BTreeMap<usize, (String, HashMap<String, usize>)> = BTreeMap::new();
    for e in entries {
        let slot = map
            .entry(level_order(&e.level))
            .or_insert_with(|| (e.level.clone(), HashMap::new()));
        *slot.1.entry(e.category.clone()).or_insert(0) += 1;
    }
    map.into_values()
        .map(|(lvl, topics)| {
            let mut t: Vec<(String, usize)> = topics.into_iter().collect();
            t.sort_by_cached_key(|(name, _)| topic_order(name));
            (lvl, t)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_database_gets_everything() {
        let mut db = Database::in_memory().unwrap();
        let rep = db.sync_wordlists(None).unwrap();
        assert!(rep.errors.is_empty());
        assert!(db.count() > 5000, "ima {}", db.count());
        // drugi poziv ne radi ništa
        let rep2 = db.sync_wordlists(None).unwrap();
        assert_eq!(rep2.added, 0);
        let all = db.load_all().unwrap();
        assert_eq!(all.first().unwrap().level, "A1");
        assert!(db.exists(&all[0].sr, &all[0].de, &all[0].category, 0).unwrap());
        assert!(!db.exists(&all[0].sr, &all[0].de, &all[0].category, all[0].id).unwrap());
    }

    #[test]
    fn old_extra_word_is_replaced_and_progress_moves() {
        let mut db = Database::in_memory().unwrap();
        db.sync_wordlists(None).unwrap();
        let before = db.count();
        // stara stručna reč koju sada pokriva lista po nivoima, sa napretkom
        db.conn.execute("DELETE FROM vocabulary WHERE de_text = 'der Tisch'", []).unwrap();
        let old = Entry {
            category: "Nameštaj".into(), kind: "Reč".into(), sr: "astal".into(),
            de: "der Tisch".into(), level: "X".into(), pos: "imenica".into(),
            ..Default::default()
        };
        db.add(&old).unwrap();
        crate::progress::record(&db, "", &old, true).unwrap();
        crate::progress::record(&db, "rod", &old, true).unwrap();
        db.set_meta("wordlists_hash", "").unwrap();

        let rep = db.sync_wordlists(None).unwrap();
        assert_eq!((rep.replaced, rep.added), (1, 1));
        assert_eq!(db.count(), before);
        let all = db.load_all().unwrap();
        let tisch: Vec<&Entry> = all.iter().filter(|e| e.de == "der Tisch").collect();
        assert_eq!(tisch.len(), 1);
        assert_eq!(tisch[0].level, "A1");
        let keys: Vec<String> = db.conn
            .prepare("SELECT key FROM progress ORDER BY key").unwrap()
            .query_map([], |r| r.get(0)).unwrap().collect::<Result<_>>().unwrap();
        let new_key = progress_key(&tisch[0].sr, "der Tisch");
        assert_eq!(keys, vec![format!("rod:{new_key}"), new_key]);
    }
}
