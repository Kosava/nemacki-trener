//! Praćenje napretka — Lajtnerov sistem kutija (spaced repetition).
//!
//!   kutija:     0      1      2      3      4       5
//!   ponovi za:  danas  1 dan  3 dana 7 dana 14 dana 30 dana
//!
//! Napredak se vezuje za par (srpski, nemački). `ns` odvaja napredak
//! različitih vežbi ("rod" za der/die/das, "popuni", "slozi", "oblik").

use std::collections::{HashMap, HashSet};

use chrono::{Duration, Local, NaiveDate};
use rusqlite::params;

use crate::db::{progress_key, Database, Result};
use crate::model::Entry;

pub const INTERVALS: [i64; 6] = [0, 1, 3, 7, 14, 30];
pub const MAX_BOX: i64 = 5;
pub const LEARNED_BOX: i64 = 3;
pub const NAMESPACES: [&str; 4] = ["rod", "popuni", "slozi", "oblik"];
/// Prefiksi ključeva u tabeli `progress` ("" = osnovno učenje).
pub const ALL_PREFIXES: [&str; 5] = ["", "rod:", "popuni:", "slozi:", "oblik:"];

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn days_text(n: i64) -> String {
    match n {
        0 => "danas".into(),
        1 => "sutra".into(),
        n => format!("za {n} dana"),
    }
}

pub fn entry_key(e: &Entry, ns: &str) -> String {
    let base = progress_key(&e.sr, &e.de);
    if ns.is_empty() { base } else { format!("{ns}:{base}") }
}

#[derive(Debug, Clone, Default)]
pub struct Progress {
    pub box_: i64,
    pub due: String,
    pub correct: i64,
    pub wrong: i64,
    pub last_seen: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub total: usize,
    pub new: usize,
    pub due: usize,
    pub learned: usize,
}

pub struct Split {
    pub due: Vec<Entry>,
    pub new: Vec<Entry>,
    pub rest: Vec<Entry>,
}

fn load(db: &Database) -> Result<HashMap<String, Progress>> {
    let mut st = db
        .conn
        .prepare("SELECT key, box, due, correct, wrong, last_seen FROM progress")?;
    let rows = st.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            Progress {
                box_: r.get(1)?,
                due: r.get(2)?,
                correct: r.get(3)?,
                wrong: r.get(4)?,
                last_seen: r.get(5)?,
            },
        ))
    })?;
    rows.collect()
}

/// Deli (jedinstvene) zapise na: za danas, nove, ostale.
/// "Za danas" je sortirano: najniža kutija i najstariji rok prvo.
pub fn split(db: &Database, ns: &str, entries: &[Entry]) -> Result<Split> {
    let today_s = today().to_string();
    let prog = load(db)?;
    let mut seen = HashSet::new();
    let mut due: Vec<(i64, String, Entry)> = Vec::new();
    let (mut new, mut rest) = (Vec::new(), Vec::new());
    for e in entries {
        let k = entry_key(e, ns);
        if !seen.insert(k.clone()) {
            continue;
        }
        match prog.get(&k) {
            None => new.push(e.clone()),
            Some(p) if p.due <= today_s => due.push((p.box_, p.due.clone(), e.clone())),
            Some(_) => rest.push(e.clone()),
        }
    }
    due.sort_by(|a, b| (a.0, &a.1).cmp(&(b.0, &b.1)));
    Ok(Split { due: due.into_iter().map(|t| t.2).collect(), new, rest })
}

pub fn stats(db: &Database, ns: &str, entries: &[Entry]) -> Result<Stats> {
    let today_s = today().to_string();
    let prog = load(db)?;
    let mut seen = HashSet::new();
    let mut s = Stats::default();
    for e in entries {
        let k = entry_key(e, ns);
        if !seen.insert(k.clone()) {
            continue;
        }
        s.total += 1;
        match prog.get(&k) {
            None => s.new += 1,
            Some(p) => {
                if p.due <= today_s {
                    s.due += 1;
                }
                if p.box_ >= LEARNED_BOX {
                    s.learned += 1;
                }
            }
        }
    }
    Ok(s)
}

pub fn get(db: &Database, ns: &str, e: &Entry) -> Result<Option<Progress>> {
    use rusqlite::OptionalExtension;
    db.conn
        .query_row(
            "SELECT box, due, correct, wrong, last_seen FROM progress WHERE key = ?1",
            [entry_key(e, ns)],
            |r| {
                Ok(Progress {
                    box_: r.get(0)?,
                    due: r.get(1)?,
                    correct: r.get(2)?,
                    wrong: r.get(3)?,
                    last_seen: r.get(4)?,
                })
            },
        )
        .optional()
}

/// Upisuje odgovor: tačan pomera reč u sledeću kutiju, netačan je vraća u kutiju 0.
pub fn record(db: &Database, ns: &str, e: &Entry, known: bool) -> Result<Progress> {
    let day = today();
    let mut p = get(db, ns, e)?.unwrap_or_default();
    if known {
        p.box_ = (p.box_ + 1).min(MAX_BOX);
        p.correct += 1;
    } else {
        p.box_ = 0;
        p.wrong += 1;
    }
    p.due = (day + Duration::days(INTERVALS[p.box_ as usize])).to_string();
    p.last_seen = day.to_string();
    let key = entry_key(e, ns);
    db.conn.execute(
        "INSERT INTO progress (key, box, due, correct, wrong, last_seen)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(key) DO UPDATE SET
            box=excluded.box, due=excluded.due, correct=excluded.correct,
            wrong=excluded.wrong, last_seen=excluded.last_seen",
        params![key, p.box_, p.due, p.correct, p.wrong, p.last_seen],
    )?;
    db.conn.execute(
        "INSERT INTO review_log (day, ns, key, correct) VALUES (?1, ?2, ?3, ?4)",
        params![day.to_string(), ns, key, known as i64],
    )?;
    Ok(p)
}

/// Briše napredak jedne vežbe (ostale ostaju).
pub fn reset(db: &Database, ns: &str) -> Result<()> {
    if ns.is_empty() {
        let cond: Vec<String> = NAMESPACES.iter().map(|n| format!("key NOT LIKE '{n}:%'")).collect();
        db.conn.execute(&format!("DELETE FROM progress WHERE {}", cond.join(" AND ")), [])?;
    } else {
        db.conn.execute("DELETE FROM progress WHERE key LIKE ?1", [format!("{ns}:%")])?;
    }
    Ok(())
}

/// Broj odgovora danas (sve vežbe) i niz dana zaredom.
pub fn today_and_streak(db: &Database) -> Result<(i64, i64)> {
    let day = today();
    let mut st = db.conn.prepare("SELECT day, COUNT(*) FROM review_log GROUP BY day")?;
    let days: HashMap<String, i64> = st
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))?
        .collect::<Result<_>>()?;
    let today_n = days.get(&day.to_string()).copied().unwrap_or(0);
    let mut streak = 0;
    let mut d = if today_n > 0 { day } else { day - Duration::days(1) };
    while days.contains_key(&d.to_string()) {
        streak += 1;
        d -= Duration::days(1);
    }
    Ok((today_n, streak))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(sr: &str, de: &str) -> Entry {
        Entry { sr: sr.into(), de: de.into(), ..Default::default() }
    }

    #[test]
    fn leitner_boxes() {
        let db = Database::in_memory().unwrap();
        let e = entry("sto", "der Tisch");
        let all = vec![e.clone(), entry("Kuća", "das Haus")];
        assert_eq!(stats(&db, "", &all).unwrap(), Stats { total: 2, new: 2, due: 0, learned: 0 });
        let p = record(&db, "", &e, true).unwrap();
        assert_eq!(p.box_, 1);
        let sp = split(&db, "", &all).unwrap();
        assert_eq!((sp.due.len(), sp.new.len(), sp.rest.len()), (0, 1, 1));
        let p = record(&db, "", &e, false).unwrap();
        assert_eq!(p.box_, 0);
        assert_eq!(split(&db, "", &all).unwrap().due.len(), 1);
        // druga vežba ima svoj napredak
        assert_eq!(stats(&db, "rod", &all).unwrap().new, 2);
        record(&db, "rod", &e, true).unwrap();
        reset(&db, "").unwrap();
        assert_eq!(stats(&db, "", &all).unwrap().new, 2);
        assert_eq!(stats(&db, "rod", &all).unwrap().new, 1);
        assert_eq!(today_and_streak(&db).unwrap(), (3, 1));
        assert_eq!(entry_key(&entry("Šuma", "die Straße"), "rod"), "rod:šuma|die strasse");
    }
}
