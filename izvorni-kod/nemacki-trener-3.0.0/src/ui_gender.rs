//! Tab "Der / Die / Das" — vežba roda imenica (napredak se vodi odvojeno).

use std::rc::Rc;
use std::time::Duration;

use rand::seq::SliceRandom;
use slint::ComponentHandle;

use crate::check::{gender_hint, split_noun};
use crate::ctl::{Ctl, Pending, Session, NEW_PER_SESSION};
use crate::model::Entry;
use crate::progress;
use crate::{AppWindow, Gender};

const NS: &str = "rod";
const AUTO_NEXT: Duration = Duration::from_millis(1300);

impl Ctl {
    pub fn connect_gender(self: Rc<Self>, ui: &AppWindow) {
        let g = ui.global::<Gender>();
        let c = self.clone();
        g.on_filter_changed(move || {
            c.gender_stop();
            c.refresh_gender_stats();
        });
        let c = self.clone();
        g.on_start(move || c.gender_start());
        let c = self.clone();
        g.on_stop(move || c.gender_stop());
        let c = self.clone();
        g.on_answer(move |a| c.clone().gender_answer(a.as_str()));
        let c = self.clone();
        g.on_next(move || c.gender_next());
        let c = self.clone();
        g.on_reset_progress(move || {
            c.ask(
                "Resetuj napredak",
                "Obrisati zapamćeni napredak vežbe Der/Die/Das?\nNapredak učenja prevoda ostaje.",
                "Resetuj",
                Pending::ResetGender,
            )
        });
    }

    fn gender_entries(&self) -> Vec<Entry> {
        self.filtered(self.ui().global::<Gender>().get_filter_index())
            .into_iter()
            .filter(|e| {
                !e.is_sentence()
                    && (e.pos == "imenica" || e.pos.is_empty())
                    && !e.forms.starts_with('(')
                    && split_noun(&e.de).is_some()
            })
            .collect()
    }

    pub fn refresh_gender_stats(&self) {
        let entries = self.gender_entries();
        let s = progress::stats(&self.st.borrow().db, NS, &entries).unwrap_or_default();
        let pct = if s.total > 0 { 100 * s.learned / s.total } else { 0 };
        self.ui().global::<Gender>().set_stats_text(
            format!(
                "Za danas: {}   ·   Novo: {}   ·   Savladano: {} / {} imenica ({}%)",
                s.due, s.new, s.learned, s.total, pct
            )
            .into(),
        );
    }

    fn gender_start(&self) {
        let ui = self.ui();
        let g = ui.global::<Gender>();
        let entries = self.gender_entries();
        let split = match progress::split(&self.st.borrow().db, NS, &entries) {
            Ok(s) => s,
            Err(e) => return self.message("Greška", &e.to_string()),
        };
        let mut rng = rand::rng();
        let pool = match g.get_mode_index() {
            0 => {
                if split.due.is_empty() {
                    let mut msg = "Za danas nema imenica za ponavljanje.".to_string();
                    if !split.new.is_empty() {
                        msg += &format!("\n\nImaš {} novih — izaberi „Nove (20)”.", split.new.len());
                    }
                    return self.message("Der / Die / Das", &msg);
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
            return self.message("Der / Die / Das", "Ovde nema novih imenica.\nIzaberi „Za ponavljanje danas” ili „Sve”.");
        }
        self.st.borrow_mut().gender = Some(Session::new(pool));
        g.set_active(true);
        self.gender_show();
    }

    fn gender_stop(&self) {
        {
            let mut st = self.st.borrow_mut();
            st.gender = None;
            st.gender_tick += 1;
        }
        let ui = self.ui();
        let g = ui.global::<Gender>();
        g.set_active(false);
        g.set_answered(false);
        g.set_progress_text("".into());
        g.set_score_text("".into());
    }

    fn gender_show(&self) {
        let ui = self.ui();
        let g = ui.global::<Gender>();
        let st = self.st.borrow();
        let Some(sess) = &st.gender else { return };
        let Some(e) = sess.current() else { return };
        let Some((art, noun)) = split_noun(&e.de) else { return };
        g.set_noun(noun.into());
        g.set_article(art.into());
        g.set_sr(e.sr.as_str().into());
        g.set_chosen("".into());
        g.set_answered(false);
        g.set_result_text("".into());
        g.set_hint("".into());
        g.set_plural(e.forms_label().into());
        g.set_progress_text(format!("Imenica {} / {}", sess.index + 1, sess.pool.len()).into());
        g.set_score_text(format!("Tačno: {}   Netačno: {}", sess.correct, sess.wrong).into());
        drop(st);
        ui.invoke_focus_page();
    }

    fn gender_answer(self: Rc<Self>, chosen: &str) {
        let ui = self.ui();
        let g = ui.global::<Gender>();
        if g.get_answered() {
            return;
        }
        let (ok, tick) = {
            let mut guard = self.st.borrow_mut();
            let st = &mut *guard;
            let Some(sess) = st.gender.as_mut() else { return };
            let Some(e) = sess.current().cloned() else { return };
            let Some((art, noun)) = split_noun(&e.de) else { return };
            let ok = chosen == art;
            let _ = progress::record(&st.db, NS, &e, ok);
            if ok {
                sess.correct += 1;
                g.set_result_text("Tačno!".into());
            } else {
                sess.wrong += 1;
                sess.pool.push(e.clone());
                g.set_result_text(format!("Netačno — tačno je {art} {noun}.").into());
            }
            g.set_hint(gender_hint(noun, art).into());
            g.set_score_text(format!("Tačno: {}   Netačno: {}", sess.correct, sess.wrong).into());
            st.gender_tick += 1;
            (ok, st.gender_tick)
        };
        g.set_chosen(chosen.into());
        g.set_result_ok(ok);
        g.set_answered(true);
        self.refresh_streak();
        if ok {
            // tačan odgovor: sam prelazi na sledeću imenicu
            let c = self.clone();
            slint::Timer::single_shot(AUTO_NEXT, move || {
                if c.st.borrow().gender_tick == tick {
                    c.gender_next();
                }
            });
        }
    }

    fn gender_next(&self) {
        if !self.ui().global::<Gender>().get_answered() {
            return;
        }
        let finished = {
            let mut st = self.st.borrow_mut();
            st.gender_tick += 1;
            let Some(sess) = st.gender.as_mut() else { return };
            sess.index += 1;
            (sess.index >= sess.pool.len()).then_some((sess.correct, sess.wrong))
        };
        match finished {
            None => self.gender_show(),
            Some((ok, bad)) => {
                self.gender_stop();
                self.refresh_gender_stats();
                self.message(
                    "Gotovo!",
                    &format!("Ukupno odgovora: {}\nTačno: {ok}\nNetačno: {bad}", ok + bad),
                );
            }
        }
    }

    pub fn gender_reset_confirmed(&self) {
        let _ = progress::reset(&self.st.borrow().db, NS);
        self.gender_stop();
        self.refresh_gender_stats();
        self.status("Napredak vežbe Der/Die/Das je obrisan.");
    }
}
