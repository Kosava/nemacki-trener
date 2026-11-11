//! Nemački Trener — srpsko-nemački rečnik i vežbe (Rust + Slint).
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod check;
mod ctl;
mod db;
mod model;
mod paths;
mod progress;
mod text;
mod ui_browse;
mod ui_gender;
mod ui_learn;
mod vuk;
mod wordlists;
#[cfg(debug_assertions)]
mod devshots;

slint::include_modules!();

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let paths = paths::Paths::resolve();
    std::fs::create_dir_all(&paths.data_dir)?;
    std::fs::create_dir_all(&paths.user_lists)?;

    // `--provera`: samo otvori bazu, dopuni rečnik i ispiši stanje (bez prozora)
    if std::env::args().any(|a| a == "--provera") {
        return provera(&paths);
    }

    let ui = AppWindow::new()?;
    let ctl = ctl::Ctl::new(&ui, paths)?;
    ctl.clone().connect(&ui);
    ctl.startup();

    #[cfg(debug_assertions)]
    devshots::maybe_run(&ui, ctl.clone());

    ui.run()?;
    Ok(())
}

fn provera(paths: &paths::Paths) -> Result<(), Box<dyn std::error::Error>> {
    let mut db = db::Database::open(&paths.db)?;
    println!("Baza: {}", paths.db.display());
    println!("Zapisa pre: {} (verzija podataka {})", db.count(), db.data_version());
    let rep = db.sync_wordlists(Some(&paths.user_lists))?;
    println!("Dodato: {}, zamenjeno starih: {}, grešaka u listama: {}", rep.added, rep.replaced, rep.errors.len());
    for e in rep.errors.iter().take(20) {
        println!("  {e}");
    }
    let all = db.load_all()?;
    let mut per_level = std::collections::BTreeMap::new();
    for e in &all {
        *per_level.entry(model::level_short(&e.level).to_string()).or_insert(0) += 1;
    }
    println!("Zapisa posle: {} {:?}", all.len(), per_level);
    let (today, streak) = progress::today_and_streak(&db)?;
    let n: i64 = db.conn.query_row("SELECT COUNT(*) FROM progress", [], |r| r.get(0))?;
    println!("Napredak: {n} reči u kutijama; danas {today} odgovora; niz {streak}");
    Ok(())
}
