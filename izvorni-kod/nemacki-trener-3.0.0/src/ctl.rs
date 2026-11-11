//! Veza između interfejsa (Slint) i logike: stanje aplikacije i događaji.

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::db::{level_topics, Database};
use crate::model::{level_label, level_short, Entry};
use crate::paths::Paths;
use crate::text::fold;
use crate::{progress, App, AppWindow, Gender, Learn, Settings};

/// Izbor nivoa / teme. (None, None) = sve.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Filter {
    pub level: Option<String>,
    pub topic: Option<String>,
}

impl Filter {
    pub fn matches(&self, e: &Entry) -> bool {
        self.level.as_ref().is_none_or(|l| *l == e.level)
            && self.topic.as_ref().is_none_or(|t| *t == e.category)
    }
}

#[derive(Default)]
pub struct BrowseState {
    pub filter: Filter,
    pub expanded: HashSet<String>,
    pub search: String,
    pub selected_id: i64,
    /// šta predstavlja svaki red u levoj listi
    pub nav: Vec<(Filter, bool)>,
    /// indeksi (u `entries`) trenutno prikazanih redova
    pub visible: Vec<usize>,
}

pub struct Session {
    pub pool: Vec<Entry>,
    pub index: usize,
    pub correct: u32,
    pub wrong: u32,
}

impl Session {
    pub fn new(pool: Vec<Entry>) -> Self {
        Self { pool, index: 0, correct: 0, wrong: 0 }
    }
    pub fn current(&self) -> Option<&Entry> {
        self.pool.get(self.index)
    }
}

pub enum Pending {
    Delete(i64),
    ResetLearn,
    ResetGender,
}

pub struct State {
    pub db: Database,
    pub paths: Paths,
    pub entries: Vec<Entry>,
    /// tekst za pretragu (bez kvačica) za svaki zapis
    pub hay: Vec<String>,
    pub browse: [BrowseState; 2],
    pub filters: Vec<Filter>,
    pub learn: Option<Session>,
    pub learn_typing: bool,
    pub learn_reverse: bool,
    pub learn_pending: Option<bool>,
    pub gender: Option<Session>,
    pub gender_tick: u64,
    pub pending: Option<Pending>,
    pub editing_id: i64,
    pub warnings: Vec<String>,
}

pub struct Ctl {
    pub ui: slint::Weak<AppWindow>,
    pub st: RefCell<State>,
}

pub const NEW_PER_SESSION: usize = 20;

pub fn strings(items: impl IntoIterator<Item = String>) -> ModelRc<SharedString> {
    let v: Vec<SharedString> = items.into_iter().map(SharedString::from).collect();
    ModelRc::new(VecModel::from(v))
}

impl Ctl {
    pub fn new(ui: &AppWindow, paths: Paths) -> Result<Rc<Self>, Box<dyn std::error::Error>> {
        let mut db = Database::open(&paths.db)?;
        let mut warnings = Vec::new();
        if db.count() > 0 && db.data_version() < crate::db::DATA_VERSION {
            warnings.push(
                "Ova baza je iz starije verzije programa (pre 2.1). Pokreni jednom Python \
                 verziju 2.2 da sredi bazu, pa je onda prekopiraj ovde. Do tada se nove \
                 reči ne dodaju."
                    .to_string(),
            );
        } else {
            let rep = db.sync_wordlists(Some(&paths.user_lists))?;
            if !rep.errors.is_empty() {
                let shown: Vec<&str> = rep.errors.iter().take(8).map(String::as_str).collect();
                warnings.push(format!(
                    "Greške u listama reči ({}):\n{}",
                    rep.errors.len(),
                    shown.join("\n")
                ));
            }
        }
        let st = State {
            db,
            paths,
            entries: Vec::new(),
            hay: Vec::new(),
            browse: [BrowseState { selected_id: -1, ..Default::default() },
                     BrowseState { selected_id: -1, ..Default::default() }],
            filters: Vec::new(),
            learn: None,
            learn_typing: false,
            learn_reverse: false,
            learn_pending: None,
            gender: None,
            gender_tick: 0,
            pending: None,
            editing_id: 0,
            warnings,
        };
        Ok(Rc::new(Self { ui: ui.as_weak(), st: RefCell::new(st) }))
    }

    pub fn ui(&self) -> AppWindow {
        self.ui.unwrap()
    }

    /// Povezuje sve događaje iz interfejsa.
    pub fn connect(self: Rc<Self>, ui: &AppWindow) {
        let app = ui.global::<App>();
        let c = self.clone();
        app.on_tab_changed(move |i| c.tab_changed(i));
        let c = self.clone();
        app.on_message_answer(move |yes| c.message_answer(yes));

        let s = ui.global::<Settings>();
        let c = self.clone();
        s.on_theme_changed(move |i| c.set_theme(i == 1));
        let c = self.clone();
        s.on_open_data_folder(move || c.open_data_folder());

        self.clone().connect_browse(ui);
        self.clone().connect_learn(ui);
        self.connect_gender(ui);
    }

    pub fn startup(&self) {
        let ui = self.ui();
        let dark = self.st.borrow().db.get_meta("ui:theme").as_deref() == Some("dark");
        ui.set_dark_mode(dark);
        let s = ui.global::<Settings>();
        s.set_theme_index(dark as i32);
        s.set_version(env!("CARGO_PKG_VERSION").into());
        {
            let st = self.st.borrow();
            s.set_db_path(st.paths.db.to_string_lossy().as_ref().into());
            s.set_user_dir(st.paths.user_lists.to_string_lossy().as_ref().into());
        }
        self.reload_entries();
        self.refresh_browse(0);
        self.refresh_learn_stats();
        self.refresh_gender_stats();
        let warn = self.st.borrow_mut().warnings.pop();
        if let Some(w) = warn {
            self.message("Napomena", &w);
        }
    }

    // ---------- podaci ----------
    pub fn reload_entries(&self) {
        let ui = self.ui();
        {
            let mut st = self.st.borrow_mut();
            st.entries = st.db.load_all().unwrap_or_default();
            st.hay = st
                .entries
                .iter()
                .map(|e| fold(&format!("{}\n{}\n{}\n{}", e.sr, e.de, e.pronunciation, e.example)))
                .collect();
            // izbor nivoa/teme za Učenje i Der/Die/Das
            let mut filters = vec![Filter::default()];
            let mut labels = vec!["Sve reči".to_string()];
            for (lvl, topics) in level_topics(st.entries.iter()) {
                filters.push(Filter { level: Some(lvl.clone()), topic: None });
                labels.push(format!("{} — sve teme", level_label(&lvl)));
                for (t, _) in topics {
                    labels.push(format!("      {} · {}", level_short(&lvl), t));
                    filters.push(Filter { level: Some(lvl.clone()), topic: Some(t) });
                }
            }
            st.filters = filters;
            let li = ui.global::<Learn>().get_filter_index().max(0) as usize;
            let gi = ui.global::<Gender>().get_filter_index().max(0) as usize;
            ui.global::<Learn>().set_filters(strings(labels.clone()));
            ui.global::<Gender>().set_filters(strings(labels));
            ui.global::<Learn>().set_filter_index(li.min(st.filters.len() - 1) as i32);
            ui.global::<Gender>().set_filter_index(gi.min(st.filters.len() - 1) as i32);

            let words = st.entries.iter().filter(|e| !e.is_sentence()).count();
            ui.global::<Settings>().set_counts_text(
                format!(
                    "U rečniku je {} zapisa: {} reči i {} rečenica.",
                    st.entries.len(),
                    words,
                    st.entries.len() - words
                )
                .into(),
            );
        }
        self.refresh_streak();
    }

    pub fn refresh_streak(&self) {
        let st = self.st.borrow();
        let (today, streak) = progress::today_and_streak(&st.db).unwrap_or((0, 0));
        let dana = match streak {
            1 => "dan".to_string(),
            _ => "dana".to_string(),
        };
        self.ui().global::<App>().set_streak_text(
            format!("Niz: {streak} {dana}   ·   Danas: {today} odgovora").into(),
        );
    }

    pub fn status(&self, text: &str) {
        self.ui().global::<App>().set_status(text.into());
    }

    // ---------- poruke ----------
    pub fn message(&self, title: &str, text: &str) {
        let ui = self.ui();
        let app = ui.global::<App>();
        app.set_message_title(title.into());
        app.set_message_text(text.into());
        app.set_message_is_question(false);
        app.set_message_visible(true);
    }

    pub fn ask(&self, title: &str, text: &str, yes: &str, action: Pending) {
        self.st.borrow_mut().pending = Some(action);
        let ui = self.ui();
        let app = ui.global::<App>();
        app.set_message_title(title.into());
        app.set_message_text(text.into());
        app.set_message_yes(yes.into());
        app.set_message_is_question(true);
        app.set_message_visible(true);
    }

    fn message_answer(&self, yes: bool) {
        let ui = self.ui();
        ui.global::<App>().set_message_visible(false);
        let action = self.st.borrow_mut().pending.take();
        if yes {
            match action {
                Some(Pending::Delete(id)) => self.delete_confirmed(id),
                Some(Pending::ResetLearn) => self.learn_reset_confirmed(),
                Some(Pending::ResetGender) => self.gender_reset_confirmed(),
                None => {}
            }
        }
        // sledeće upozorenje, ako ih ima još
        let warn = self.st.borrow_mut().warnings.pop();
        if let Some(w) = warn {
            self.message("Napomena", &w);
        } else {
            ui.invoke_focus_page();
        }
    }

    // ---------- tabovi i podešavanja ----------
    pub fn tab_changed(&self, tab: i32) {
        match tab {
            0 | 1 => self.refresh_browse(tab as usize),
            2 => self.refresh_learn_stats(),
            3 => self.refresh_gender_stats(),
            _ => {}
        }
    }

    pub fn set_theme(&self, dark: bool) {
        let ui = self.ui();
        ui.set_dark_mode(dark);
        ui.global::<Settings>().set_theme_index(dark as i32);
        let _ = self.st.borrow().db.set_meta("ui:theme", if dark { "dark" } else { "light" });
    }

    fn open_data_folder(&self) {
        let dir = self.st.borrow().paths.data_dir.clone();
        let cmd = if cfg!(windows) {
            "explorer"
        } else if cfg!(target_os = "macos") {
            "open"
        } else {
            "xdg-open"
        };
        if std::process::Command::new(cmd).arg(&dir).spawn().is_err() {
            self.status(&format!("Folder: {}", dir.display()));
        }
    }

    /// Zapisi koji odgovaraju izboru u padajućoj listi (Učenje, Der/Die/Das).
    pub fn filtered(&self, filter_index: i32) -> Vec<Entry> {
        let st = self.st.borrow();
        let f = st.filters.get(filter_index.max(0) as usize).cloned().unwrap_or_default();
        st.entries.iter().filter(|e| f.matches(e)).cloned().collect()
    }
}
