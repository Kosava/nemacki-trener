//! Učitavanje rečnika po nivoima iz tekst fajlova (data/*.txt).
//!
//! Nivo se čita iz imena fajla (a1_1.txt => A1). Format reda:
//!   N | der Tisch | die Tische | sto | Der Tisch ist neu. | Sto je nov.
//!   V | gehen | ist gegangen | ići | Ich gehe nach Hause. | Idem kući.
//!   A | groß | | velik | Das Haus ist groß. | Kuća je velika.
//!   W | heute | | danas | Heute ist Montag. | Danas je ponedeljak.
//!   S | Wie geht's? | Kako si?
//! '## Naziv teme' započinje novu temu, '#' je komentar.

use std::collections::HashMap;
use std::path::Path;

use crate::model::{Entry, TYPE_SENTENCE, TYPE_WORD};
use crate::vuk::to_vuk;

include!(concat!(env!("OUT_DIR"), "/data_files.rs"));

pub const LEVELS: [&str; 4] = ["A1", "A2", "B1", "B2"];

#[derive(Debug, Clone)]
pub struct WordRow {
    pub level: String,
    pub topic: String,
    pub pos: &'static str,
    pub de: String,
    pub form: String,
    pub sr: String,
    pub ex_de: String,
    pub ex_sr: String,
    pub source: String,
}

impl WordRow {
    pub fn is_sentence(&self) -> bool {
        self.pos == "rečenica"
    }

    pub fn example(&self) -> String {
        if self.is_sentence() || self.ex_de.is_empty() {
            String::new()
        } else if self.ex_sr.is_empty() {
            self.ex_de.clone()
        } else {
            format!("{} — {}", self.ex_de, self.ex_sr)
        }
    }

    pub fn to_entry(&self) -> Entry {
        Entry {
            id: 0,
            category: self.topic.clone(),
            kind: if self.is_sentence() { TYPE_SENTENCE } else { TYPE_WORD }.to_string(),
            sr: self.sr.clone(),
            de: self.de.clone(),
            pronunciation: to_vuk(&self.de),
            example: self.example(),
            level: self.level.clone(),
            pos: self.pos.to_string(),
            forms: self.form.clone(),
        }
    }
}

fn pos_name(code: &str) -> Option<&'static str> {
    Some(match code {
        "N" => "imenica",
        "V" => "glagol",
        "A" => "pridev",
        "W" => "ostalo",
        "S" => "rečenica",
        _ => return None,
    })
}

fn has_article(de: &str) -> bool {
    ["der ", "die ", "das "]
        .iter()
        .any(|a| de.starts_with(a) && de.len() > a.len() && !de[a.len()..].starts_with(' '))
}

pub fn valid_perfekt(form: &str) -> bool {
    ["hat/ist ", "hat ", "ist "].iter().any(|p| {
        form.strip_prefix(p)
            .is_some_and(|rest| rest.chars().next().is_some_and(|c| !c.is_whitespace()))
    })
}

fn level_of(file_name: &str) -> String {
    let stem = file_name.strip_suffix(".txt").unwrap_or(file_name);
    stem.split('_').next().unwrap_or("").to_uppercase()
}

pub fn parse(file_name: &str, content: &str) -> (Vec<WordRow>, Vec<String>) {
    let level = level_of(file_name);
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut topic = String::new();
    for (n, raw) in content.lines().enumerate() {
        let line = raw.trim();
        let src = format!("{}:{}", file_name, n + 1);
        if line.is_empty() {
            continue;
        }
        if line.starts_with("##") {
            topic = line.trim_start_matches('#').trim().to_string();
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = line.split('|').map(str::trim).collect();
        let code = parts[0];
        let Some(pos) = pos_name(code) else {
            errors.push(format!("{src}: nepoznata vrsta '{code}'"));
            continue;
        };
        if topic.is_empty() {
            errors.push(format!("{src}: red pre prve teme"));
            continue;
        }
        let row = |de: &str, form: &str, sr: &str, ex_de: &str, ex_sr: &str| WordRow {
            level: level.clone(),
            topic: topic.clone(),
            pos,
            de: de.into(),
            form: form.into(),
            sr: sr.into(),
            ex_de: ex_de.into(),
            ex_sr: ex_sr.into(),
            source: src.clone(),
        };
        if code == "S" {
            if parts.len() != 3 || parts[1].is_empty() || parts[2].is_empty() {
                errors.push(format!("{src}: rečenica mora imati 3 polja"));
                continue;
            }
            rows.push(row(parts[1], "", parts[2], "", ""));
            continue;
        }
        if parts.len() != 6 {
            errors.push(format!("{src}: očekivano 6 polja, ima {}", parts.len()));
            continue;
        }
        let (de, form, sr, ex_de, ex_sr) = (parts[1], parts[2], parts[3], parts[4], parts[5]);
        if de.is_empty() || sr.is_empty() || ex_de.is_empty() || ex_sr.is_empty() {
            errors.push(format!("{src}: prazno obavezno polje"));
            continue;
        }
        match code {
            "N" => {
                if !has_article(de) {
                    errors.push(format!("{src}: imenica bez člana '{de}'"));
                }
                if form.is_empty() {
                    errors.push(format!("{src}: imenica bez množine (stavi — ako je nema)"));
                }
            }
            "V" => {
                if !valid_perfekt(form) {
                    errors.push(format!("{src}: Perfekt mora biti 'hat …' ili 'ist …': '{form}'"));
                }
            }
            _ => {
                if !form.is_empty() {
                    errors.push(format!("{src}: oblik je dozvoljen samo za N i V"));
                }
            }
        }
        if code != "N" && code != "W" && has_article(de) {
            errors.push(format!("{src}: '{de}' izgleda kao imenica, a vrsta je {code}"));
        }
        rows.push(row(de, form, sr, ex_de, ex_sr));
    }
    (rows, errors)
}

fn file_order(name: &str) -> (usize, String) {
    let lvl = level_of(name);
    (LEVELS.iter().position(|l| *l == lvl).unwrap_or(99), name.to_string())
}

/// Ugrađeni fajlovi + (opciono) korisnikovi fajlovi iz `user_dir`.
/// Fajl istog imena u `user_dir` zamenjuje ugrađeni.
pub fn sources(user_dir: Option<&Path>) -> Vec<(String, String)> {
    let mut map: HashMap<String, String> = DATA_FILES
        .iter()
        .map(|(n, c)| (n.to_string(), c.to_string()))
        .collect();
    if let Some(dir) = user_dir {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let name = e.file_name().to_string_lossy().into_owned();
                if name.ends_with(".txt") {
                    if let Ok(c) = std::fs::read_to_string(e.path()) {
                        map.insert(name, c);
                    }
                }
            }
        }
    }
    let mut v: Vec<(String, String)> = map.into_iter().collect();
    v.sort_by_key(|(n, _)| file_order(n));
    v
}

/// Učitava sve; duplikat (isti nemački tekst i vrsta) zadržava prvo
/// pojavljivanje — najniži nivo.
pub fn load_all(user_dir: Option<&Path>) -> (Vec<WordRow>, Vec<String>, u64) {
    let mut rows = Vec::new();
    let mut errors = Vec::new();
    let mut hash: u64 = 0xcbf29ce484222325; // FNV-1a
    for (name, content) in sources(user_dir) {
        for b in name.bytes().chain(content.bytes()) {
            hash ^= b as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        let (r, e) = parse(&name, &content);
        rows.extend(r);
        errors.extend(e);
    }
    let mut seen: HashMap<(String, &'static str), String> = HashMap::new();
    let mut unique = Vec::with_capacity(rows.len());
    for r in rows {
        let k = (r.de.clone(), r.pos);
        if let Some(first) = seen.get(&k) {
            errors.push(format!("{}: duplikat '{}' (već u {})", r.source, r.de, first));
            continue;
        }
        seen.insert(k, r.source.clone());
        unique.push(r);
    }
    (unique, errors, hash)
}

/// Starije reči (lekcije i stručne teme) koje nisu u listama po nivoima.
pub fn legacy_entries() -> Vec<Entry> {
    LEGACY_TSV
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let p: Vec<&str> = l.split('\t').collect();
            (p.len() == 9).then(|| Entry {
                id: 0,
                level: p[0].into(),
                category: p[1].into(),
                kind: p[2].into(),
                sr: p[3].into(),
                de: p[4].into(),
                pronunciation: p[5].into(),
                example: p[6].into(),
                pos: p[7].into(),
                forms: p[8].into(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_data_is_valid() {
        let (rows, errors, _) = load_all(None);
        for e in errors.iter().take(30) {
            eprintln!("{e}");
        }
        assert!(errors.is_empty(), "{} grešaka u data/*.txt", errors.len());
        assert!(rows.len() > 3500);
        assert!(legacy_entries().len() > 1000);
    }
}
