//! Tabovi "Rečnik" i "Fraze i rečenice" + dijalog za unos / izmenu.

use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, VecModel};

use crate::ctl::{strings, Ctl, Filter, Pending};
use crate::db::level_topics;
use crate::model::{level_label, Entry, LEVEL_ORDER, TOPIC_ORDER, TYPE_SENTENCE, TYPE_WORD};
use crate::progress::ALL_PREFIXES;
use crate::text::fold;
use crate::vuk::to_vuk;
use crate::{db, wordlists, App, AppWindow, Browse, Editor, NavItem, WordItem};

const POS_NAMES: [&str; 4] = ["imenica", "glagol", "pridev", "ostalo"];

fn word_item(e: &Entry) -> WordItem {
    let (ex_de, ex_sr) = e.example_parts();
    WordItem {
        id: e.id as i32,
        de: e.de.as_str().into(),
        sr: e.sr.as_str().into(),
        pron: e.pronunciation.as_str().into(),
        level: e.level.as_str().into(),
        topic: e.category.as_str().into(),
        forms: e.forms_label().into(),
        example_de: ex_de.into(),
        example_sr: ex_sr.into(),
    }
}

impl Ctl {
    pub fn connect_browse(self: Rc<Self>, ui: &AppWindow) {
        let b = ui.global::<Browse>();
        let c = self.clone();
        b.on_nav_clicked(move |i| c.nav_clicked(i as usize));
        let c = self.clone();
        b.on_search_edited(move |t| c.search_edited(t.as_str()));
        let c = self.clone();
        b.on_row_clicked(move |i| c.row_clicked(i as usize));
        let c = self.clone();
        b.on_add_clicked(move || c.open_editor(None));
        let c = self.clone();
        b.on_edit_clicked(move || {
            let id = c.st.borrow().browse[c.tab()].selected_id;
            if id >= 0 {
                c.open_editor(Some(id));
            }
        });
        let c = self.clone();
        b.on_delete_clicked(move || c.delete_clicked());

        let e = ui.global::<Editor>();
        let c = self.clone();
        e.on_save(move || c.editor_save());
        let c = self.clone();
        e.on_cancel(move || c.ui().global::<Editor>().set_visible(false));
        let c = self.clone();
        e.on_auto_pron(move || {
            let ed = c.ui();
            let ed = ed.global::<Editor>();
            ed.set_pron(to_vuk(ed.get_de().as_str()).into());
        });
    }

    /// 0 = Rečnik, 1 = Fraze i rečenice
    fn tab(&self) -> usize {
        (self.ui().global::<App>().get_tab() == 1) as usize
    }

    pub fn refresh_browse(&self, tab: usize) {
        let ui = self.ui();
        let b = ui.global::<Browse>();
        let mut guard = self.st.borrow_mut();
        let st = &mut *guard;
        let sentences = tab == 1;
        let bs = &mut st.browse[tab];

        // ---- leva lista: nivoi i teme ----
        let of_type: Vec<&Entry> =
            st.entries.iter().filter(|e| e.is_sentence() == sentences).collect();
        let mut nav_items = vec![NavItem {
            label: if sentences { "Sve rečenice" } else { "Sve reči" }.into(),
            count: of_type.len() as i32,
            is_level: true,
            level: "*".into(),
            expanded: false,
            selected: bs.filter == Filter::default(),
        }];
        let mut nav = vec![(Filter::default(), true)];
        for (lvl, topics) in level_topics(of_type.iter().copied()) {
            let total: usize = topics.iter().map(|t| t.1).sum();
            let expanded = bs.expanded.contains(&lvl);
            let f = Filter { level: Some(lvl.clone()), topic: None };
            nav_items.push(NavItem {
                label: level_label(&lvl).chars().skip(2).collect::<String>().into(),
                count: total as i32,
                is_level: true,
                level: lvl.as_str().into(),
                expanded,
                selected: bs.filter == f,
            });
            nav.push((f, true));
            if expanded {
                for (t, n) in topics {
                    let f = Filter { level: Some(lvl.clone()), topic: Some(t.clone()) };
                    nav_items.push(NavItem {
                        label: t.into(),
                        count: n as i32,
                        is_level: false,
                        level: lvl.as_str().into(),
                        expanded: false,
                        selected: bs.filter == f,
                    });
                    nav.push((f, false));
                }
            }
        }
        bs.nav = nav;

        // ---- redovi ----
        let needle = fold(bs.search.trim());
        bs.visible = st
            .entries
            .iter()
            .enumerate()
            .filter(|(i, e)| {
                e.is_sentence() == sentences
                    && bs.filter.matches(e)
                    && (needle.is_empty() || st.hay[*i].contains(&needle))
            })
            .map(|(i, _)| i)
            .collect();
        let rows: Vec<WordItem> = bs.visible.iter().map(|&i| word_item(&st.entries[i])).collect();

        let selected = st.entries.iter().find(|e| e.id == bs.selected_id && bs.selected_id >= 0);
        match selected {
            Some(e) if bs.visible.iter().any(|&i| st.entries[i].id == e.id) => {
                b.set_selected(word_item(e));
                b.set_selected_id(e.id as i32);
            }
            _ => {
                bs.selected_id = -1;
                b.set_selected_id(-1);
            }
        }

        let n = rows.len();
        b.set_count_text(
            match n {
                1 => "1 zapis".to_string(),
                _ => format!("{n} zapisa"),
            }
            .into(),
        );
        b.set_sentences(sentences);
        b.set_search(bs.search.as_str().into());
        b.set_nav(ModelRc::new(VecModel::from(nav_items)));
        b.set_rows(ModelRc::new(VecModel::from(rows)));
        ui.global::<App>()
            .set_status(format!("Ukupno zapisa u rečniku: {}", st.entries.len()).into());
    }

    fn nav_clicked(&self, i: usize) {
        let tab = self.tab();
        {
            let mut st = self.st.borrow_mut();
            let bs = &mut st.browse[tab];
            let Some((f, is_level)) = bs.nav.get(i).cloned() else { return };
            if is_level {
                if let Some(lvl) = &f.level {
                    // klik na već izabran nivo ga skuplja; inače ga širi
                    if bs.filter == f && bs.expanded.contains(lvl) {
                        bs.expanded.remove(lvl);
                    } else {
                        bs.expanded.insert(lvl.clone());
                    }
                }
            }
            bs.filter = f;
        }
        self.refresh_browse(tab);
    }

    fn search_edited(&self, text: &str) {
        let tab = self.tab();
        self.st.borrow_mut().browse[tab].search = text.to_string();
        self.refresh_browse(tab);
    }

    fn row_clicked(&self, i: usize) {
        let tab = self.tab();
        let ui = self.ui();
        let b = ui.global::<Browse>();
        let mut st = self.st.borrow_mut();
        let Some(&idx) = st.browse[tab].visible.get(i) else { return };
        let e = st.entries[idx].clone();
        st.browse[tab].selected_id = e.id;
        b.set_selected(word_item(&e));
        b.set_selected_id(e.id as i32);
    }

    // ---------- unos / izmena ----------
    fn open_editor(&self, id: Option<i64>) {
        let ui = self.ui();
        let ed = ui.global::<Editor>();
        let tab = self.tab();
        let st = self.st.borrow();

        let mut topics: Vec<String> = TOPIC_ORDER.iter().map(|t| t.to_string()).collect();
        for e in &st.entries {
            if !topics.contains(&e.category) {
                topics.push(e.category.clone());
            }
        }

        let entry = id.and_then(|id| st.entries.iter().find(|e| e.id == id).cloned());
        let e = entry.clone().unwrap_or_else(|| {
            let f = &st.browse[tab].filter;
            Entry {
                level: f.level.clone().unwrap_or_else(|| "A1".into()),
                category: f.topic.clone().unwrap_or_default(),
                kind: if tab == 1 { TYPE_SENTENCE } else { TYPE_WORD }.into(),
                pos: if tab == 1 { "rečenica" } else { "imenica" }.into(),
                ..Default::default()
            }
        });
        drop(st);
        self.st.borrow_mut().editing_id = entry.as_ref().map_or(0, |e| e.id);

        let (ex_de, ex_sr) = e.example_parts();
        ed.set_title(
            match (&entry, e.is_sentence()) {
                (Some(_), true) => "Izmena rečenice",
                (Some(_), false) => "Izmena reči",
                (None, true) => "Nova rečenica",
                (None, false) => "Nova reč",
            }
            .into(),
        );
        ed.set_level_index(LEVEL_ORDER.iter().position(|l| *l == e.level).unwrap_or(5) as i32);
        ed.set_topic_index(topics.iter().position(|t| *t == e.category).map_or(-1, |i| i as i32));
        ed.set_topics(strings(topics));
        ed.set_topic(e.category.as_str().into());
        ed.set_sentence(e.is_sentence());
        ed.set_pos_index(POS_NAMES.iter().position(|p| *p == e.pos).unwrap_or(3) as i32);
        ed.set_de(e.de.as_str().into());
        ed.set_sr(e.sr.as_str().into());
        ed.set_pron(e.pronunciation.as_str().into());
        ed.set_forms(e.forms.as_str().into());
        ed.set_example_de(ex_de.into());
        ed.set_example_sr(ex_sr.into());
        ed.set_error("".into());
        ed.set_visible(true);
        ui.invoke_focus_editor();
    }

    fn editor_save(&self) {
        let ui = self.ui();
        let ed = ui.global::<Editor>();
        let get = |s: slint::SharedString| s.trim().to_string();
        let sentence = ed.get_sentence();
        let pos_i = ed.get_pos_index().clamp(0, 3) as usize;
        let de = get(ed.get_de());
        let sr = get(ed.get_sr());
        let topic = get(ed.get_topic());
        let level = LEVEL_ORDER[ed.get_level_index().clamp(0, 5) as usize].to_string();
        let mut forms = if !sentence && pos_i < 2 { get(ed.get_forms()) } else { String::new() };
        let (ex_de, ex_sr) = (get(ed.get_example_de()), get(ed.get_example_sr()));

        let err = if de.is_empty() || sr.is_empty() {
            "Nemački i srpski tekst su obavezni."
        } else if topic.is_empty() {
            "Izaberi ili upiši temu."
        } else if !sentence && pos_i == 1 && !forms.is_empty() && !wordlists::valid_perfekt(&forms) {
            "Perfekt mora počinjati sa „hat ”, „ist ” ili „hat/ist ” (npr. ist gegangen)."
        } else {
            ""
        };
        if !err.is_empty() {
            ed.set_error(err.into());
            return;
        }
        if !sentence && pos_i == 0 && !forms.is_empty() && forms != "—" && !forms.starts_with('(')
            && !forms.starts_with("die ")
        {
            forms = format!("die {forms}"); // množina se uvek piše sa "die"
        }

        let id = self.st.borrow().editing_id;
        let mut pron = get(ed.get_pron());
        if pron.is_empty() {
            pron = to_vuk(&de);
        }
        let entry = Entry {
            id,
            category: topic,
            kind: if sentence { TYPE_SENTENCE } else { TYPE_WORD }.into(),
            sr,
            de,
            pronunciation: pron,
            example: if sentence || ex_de.is_empty() {
                String::new()
            } else if ex_sr.is_empty() {
                ex_de
            } else {
                format!("{ex_de} — {ex_sr}")
            },
            level,
            pos: if sentence { "rečenica".into() } else { POS_NAMES[pos_i].into() },
            forms,
        };

        let result: Result<i64, String> = (|| {
            let st = self.st.borrow();
            if st.db.exists(&entry.sr, &entry.de, &entry.category, id).map_err(|e| e.to_string())? {
                return Err("Ovaj zapis već postoji u toj temi.".to_string());
            }
            if id > 0 {
                let old = st.entries.iter().find(|e| e.id == id).cloned();
                st.db.update(&entry).map_err(|e| e.to_string())?;
                // ako je promenjen tekst, napredak ide na novi ključ
                if let Some(old) = old {
                    let (ok, nk) = (db::progress_key(&old.sr, &old.de), db::progress_key(&entry.sr, &entry.de));
                    if ok != nk {
                        for ns in ALL_PREFIXES {
                            let _ = st.db.conn.execute(
                                "UPDATE OR IGNORE progress SET key = ?1 WHERE key = ?2",
                                [format!("{ns}{nk}"), format!("{ns}{ok}")],
                            );
                        }
                    }
                }
                Ok(id)
            } else {
                st.db.add(&entry).map_err(|e| e.to_string())
            }
        })();

        match result {
            Err(msg) => ed.set_error(msg.into()),
            Ok(new_id) => {
                ed.set_visible(false);
                let tab = entry.is_sentence() as usize;
                {
                    let mut st = self.st.borrow_mut();
                    let bs = &mut st.browse[tab];
                    bs.selected_id = new_id;
                    // da se novi zapis sigurno vidi u listi
                    if !bs.filter.matches(&entry) {
                        bs.filter = Filter { level: Some(entry.level.clone()), topic: Some(entry.category.clone()) };
                    }
                    bs.expanded.insert(entry.level.clone());
                    bs.search.clear();
                }
                self.reload_entries();
                ui.global::<App>().set_tab(tab as i32);
                self.refresh_browse(tab);
                self.status(&format!("Sačuvano: {} — {}", entry.de, entry.sr));
            }
        }
    }

    fn delete_clicked(&self) {
        let tab = self.tab();
        let (id, text) = {
            let st = self.st.borrow();
            let id = st.browse[tab].selected_id;
            let Some(e) = st.entries.iter().find(|e| e.id == id) else { return };
            (id, format!("{} — {}", e.de, e.sr))
        };
        self.ask("Brisanje", &format!("Obrisati zapis?\n\n{text}"), "Obriši", Pending::Delete(id));
    }

    pub fn delete_confirmed(&self, id: i64) {
        let tab = self.tab();
        {
            let mut st = self.st.borrow_mut();
            let _ = st.db.delete(id);
            st.browse[tab].selected_id = -1;
        }
        self.reload_entries();
        self.refresh_browse(tab);
        self.status("Zapis je obrisan.");
    }
}
