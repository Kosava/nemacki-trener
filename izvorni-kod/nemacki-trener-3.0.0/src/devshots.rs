//! Samo za razvoj: `NT_SHOTS=folder` snima slike ekrana kroz nekoliko koraka.

use std::io::Write;
use std::rc::Rc;
use std::time::Duration;

use slint::ComponentHandle;

use crate::ctl::Ctl;
use crate::{App, AppWindow, Browse, Editor, Gender, Learn};

fn snap(ui: &AppWindow, dir: &str, name: &str) {
    match ui.window().take_snapshot() {
        Ok(buf) => {
            let path = format!("{dir}/{name}.ppm");
            let mut f = std::fs::File::create(&path).unwrap();
            write!(f, "P6\n{} {}\n255\n", buf.width(), buf.height()).unwrap();
            let rgb: Vec<u8> = buf.as_bytes().chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
            f.write_all(&rgb).unwrap();
        }
        Err(e) => eprintln!("snapshot {name}: {e}"),
    }
}

pub fn maybe_run(ui: &AppWindow, _ctl: Rc<Ctl>) {
    let Ok(dir) = std::env::var("NT_SHOTS") else { return };
    let weak = ui.as_weak();
    let step = std::cell::Cell::new(0);
    let timer = Box::leak(Box::new(slint::Timer::default()));
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(500), move || {
        let ui = weak.unwrap();
        let n = step.get();
        step.set(n + 1);
        let (app, b, l, g) = (ui.global::<App>(), ui.global::<Browse>(), ui.global::<Learn>(), ui.global::<Gender>());
        let tab = |i: i32| {
            app.set_tab(i);
            app.invoke_tab_changed(i);
        };
        match n {
            0 => {}
            1 => { snap(&ui, &dir, "01_recnik"); b.invoke_nav_clicked(1); }
            2 => { b.invoke_nav_clicked(5); b.invoke_row_clicked(3); }
            3 => { snap(&ui, &dir, "02_tema"); b.invoke_search_edited("kase".into()); }
            4 => { snap(&ui, &dir, "03_pretraga"); b.invoke_search_edited("".into()); b.invoke_row_clicked(2); b.invoke_edit_clicked(); }
            5 => { snap(&ui, &dir, "04_izmena"); ui.global::<Editor>().invoke_cancel(); tab(1); }
            6 => { snap(&ui, &dir, "05_fraze"); tab(2); }
            7 => { l.invoke_start(); }
            8 => { snap(&ui, &dir, "06_ucenje"); l.invoke_reveal(); }
            9 => { snap(&ui, &dir, "07_odgovor"); l.invoke_grade(true); l.invoke_stop(); l.set_answer_index(1); l.invoke_start(); }
            10 => { l.set_answer("xyz".into()); l.invoke_check(); }
            11 => { snap(&ui, &dir, "08_kucanje"); l.invoke_stop(); tab(3); g.invoke_start(); }
            12 => { snap(&ui, &dir, "09_rod"); g.invoke_answer("die".into()); }
            13 => { snap(&ui, &dir, "10_rod_odgovor"); tab(4); }
            14 => { snap(&ui, &dir, "11_podesavanja"); ui.set_dark_mode(true); tab(0); }
            15 => { snap(&ui, &dir, "12_tamna"); tab(3); }
            16 => { snap(&ui, &dir, "13_tamna_rod"); tab(2); l.set_answer_index(0); l.invoke_start(); l.invoke_reveal(); }
            17 => { snap(&ui, &dir, "14_tamna_ucenje"); }
            _ => { let _ = slint::quit_event_loop(); }
        }
    });
}
