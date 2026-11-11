//! Tab "Učenje" — kartice sa ponavljanjem po kutijama.

use std::rc::Rc;

use rand::seq::SliceRandom;
use slint::ComponentHandle;

use crate::check::{check_answer, Status};
use crate::ctl::{Ctl, Pending, Session, NEW_PER_SESSION};
use crate::model::{level_short, Entry};
use crate::progress::{self, days_text, INTERVALS};
use crate::{AppWindow, Learn};

impl Ctl {
    pub fn connect_learn(self: Rc<Self>, ui: &AppWindow) {
        let l = ui.global::<Learn>();
        let c = self.clone();
        l.on_filter_changed(move || {
            c.learn_stop();
            c.refresh_learn_stats();
        });
        let c = self.clone();
        l.on_start(move || c.learn_start());
        let c = self.clone();
        l.on_stop(move || c.learn_stop());
        let c = self.clone();
        l.on_reveal(move || c.ui().global::<Learn>().set_revealed(true));
        let c = self.clone();
        l.on_grade(move |known| c.learn_grade(known));
        let c = self.clone();
        l.on_check(move || c.learn_check());
        let c = self.clone();
        l.on_next(move || {
            let pending = c.st.borrow_mut().learn_pending.take();
            if let Some(known) = pending {
                c.learn_grade(known);
            }
        });
        let c = self.clone();
        l.on_override_correct(move || {
            c.st.borrow_mut().learn_pending = Some(true);
            let ui = c.ui();
            let l = ui.global::<Learn>();
            l.set_result_text("Računa se kao tačno.".into());
            l.set_result_kind(1);
            l.set_can_override(false);
        });
        let c = self.clone();
        l.on_reset_progress(move || {
            c.ask(
                "Resetuj napredak",
                "Obrisati zapamćeni napredak učenja prevoda (kutije)?\n\
                 Napredak vežbe Der/Die/Das ostaje.",
                "Resetuj",
                Pending::ResetLearn,
            )
        });
    }

    fn learn_entries(&self) -> Vec<Entry> {
        let ui = self.ui();
        let l = ui.global::<Learn>();
        let content = l.get_content_index();
        self.filtered(l.get_filter_index())
            .into_iter()
            .filter(|e| match content {
                0 => !e.is_sentence(),
                1 => e.is_sentence(),
                _ => true,
            })
            .collect()
    }

    pub fn refresh_learn_stats(&self) {
        let entries = self.learn_entries();
        let s = progress::stats(&self.st.borrow().db, "", &entries).unwrap_or_default();
        let pct = if s.total > 0 { 100 * s.learned / s.total } else { 0 };
        self.ui().global::<Learn>().set_stats_text(
            format!(
                "Za danas: {}   ·   Novo: {}   ·   Savladano: {} / {} ({}%)",
                s.due, s.new, s.learned, s.total, pct
            )
            .into(),
        );
    }

    fn learn_start(&self) {
        let ui = self.ui();
        let l = ui.global::<Learn>();
        let entries = self.learn_entries();
        let split = match progress::split(&self.st.borrow().db, "", &entries) {
            Ok(s) => s,
            Err(e) => return self.message("Greška", &e.to_string()),
        };
        let mut rng = rand::rng();
        let pool = match l.get_mode_index() {
            0 => {
                if split.due.is_empty() {
                    let mut msg = "Za danas nema ništa za ponavljanje.".to_string();
                    if !split.new.is_empty() {
                        msg += &format!("\n\nImaš {} novih — izaberi „Nove (20)”.", split.new.len());
                    }
                    return self.message("Učenje", &msg);
                }
                split.due
            }
            1 => {
                let mut new = split.new;
                new.shuffle(&mut rng);
                new.truncate(NEW_PER_SESSION);
                new
            }
            _ => {
                let mut all = split.due;
                all.extend(split.new);
                all.extend(split.rest);
                all.shuffle(&mut rng);
                all
            }
        };
        if pool.is_empty() {
            return self.message("Učenje", "Ovde nema novih reči — sve si već video.\nIzaberi „Za ponavljanje danas” ili „Sve”.");
        }
        {
            let mut st = self.st.borrow_mut();
            st.learn = Some(Session::new(pool));
            st.learn_typing = l.get_answer_index() == 1;
            st.learn_reverse = l.get_direction_index() == 1;
            st.learn_pending = None;
        }
        l.set_typing(l.get_answer_index() == 1);
        l.set_active(true);
        self.learn_show();
    }

    fn learn_stop(&self) {
        self.st.borrow_mut().learn = None;
        let ui = self.ui();
        let l = ui.global::<Learn>();
        l.set_active(false);
        l.set_progress_text("".into());
        l.set_score_text("".into());
    }

    fn learn_show(&self) {
        let ui = self.ui();
        let l = ui.global::<Learn>();
        let st = self.st.borrow();
        let Some(sess) = &st.learn else { return };
        let Some(e) = sess.current() else { return };
        let (ex_de, ex_sr) = e.example_parts();
        let place = format!("{} · {}", level_short(&e.level), e.category);
        let forms = e.forms_label();
        if st.learn_reverse {
            l.set_front(e.de.as_str().into());
            l.set_front_note(format!("[{}]", e.pronunciation).into());
            l.set_back(e.sr.as_str().into());
            l.set_pron(forms.into());
        } else {
            l.set_front(e.sr.as_str().into());
            l.set_front_note(place.into());
            l.set_back(e.de.as_str().into());
            let mut detail = format!("[{}]", e.pronunciation);
            if !forms.is_empty() {
                detail += &format!("    ·    {forms}");
            }
            l.set_pron(detail.into());
        }
        l.set_example_de(ex_de.into());
        l.set_example_sr(ex_sr.into());
        l.set_revealed(false);
        l.set_result_text("".into());
        l.set_result_kind(0);
        l.set_can_override(false);
        l.set_answer("".into());
        l.set_progress_text(format!("Kartica {} / {}", sess.index + 1, sess.pool.len()).into());
        l.set_score_text(format!("Tačno: {}   Netačno: {}", sess.correct, sess.wrong).into());
        drop(st);
        ui.invoke_focus_page();
    }

    fn learn_check(&self) {
        let ui = self.ui();
        let l = ui.global::<Learn>();
        let answer = l.get_answer().to_string();
        if answer.trim().is_empty() || l.get_revealed() {
            return;
        }
        let expected = {
            let st = self.st.borrow();
            let Some(e) = st.learn.as_ref().and_then(|s| s.current()) else { return };
            if st.learn_reverse { e.sr.clone() } else { e.de.clone() }
        };
        let res = check_answer(&answer, &expected);
        self.st.borrow_mut().learn_pending = Some(res.correct());
        l.set_result_text(res.message.as_str().into());
        l.set_result_kind(if res.correct() { 1 } else { 2 });
        l.set_can_override(matches!(res.status, Status::Wrong));
        l.set_revealed(true);
    }

    fn learn_grade(&self, known: bool) {
        let finished = {
            let mut guard = self.st.borrow_mut();
            let st = &mut *guard;
            let Some(sess) = st.learn.as_mut() else { return };
            let Some(e) = sess.current().cloned() else { return };
            let msg = match progress::record(&st.db, "", &e, known) {
                Ok(p) => format!(
                    "{}: kutija {}, sledeće ponavljanje {}",
                    e.de,
                    p.box_,
                    days_text(INTERVALS[p.box_ as usize])
                ),
                Err(err) => format!("Greška pri upisu napretka: {err}"),
            };
            if known {
                sess.correct += 1;
            } else {
                sess.wrong += 1;
                sess.pool.push(e); // vraća se na kraj sesije
            }
            sess.index += 1;
            let done = sess.index >= sess.pool.len();
            let summary = (sess.correct, sess.wrong);
            self.ui().global::<crate::App>().set_status(msg.into());
            done.then_some(summary)
        };
        self.refresh_streak();
        match finished {
            None => self.learn_show(),
            Some((ok, bad)) => {
                self.learn_stop();
                self.refresh_learn_stats();
                self.message(
                    "Gotovo!",
                    &format!("Ukupno odgovora: {}\nTačno: {ok}\nNetačno: {bad}", ok + bad),
                );
            }
        }
    }

    pub fn learn_reset_confirmed(&self) {
        let _ = progress::reset(&self.st.borrow().db, "");
        self.learn_stop();
        self.refresh_learn_stats();
        self.status("Napredak učenja je obrisan.");
    }
}
