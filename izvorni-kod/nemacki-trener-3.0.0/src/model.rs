//! Zapis u rečniku.

pub const TYPE_WORD: &str = "Reč";
pub const TYPE_SENTENCE: &str = "Rečenica";

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Entry {
    pub id: i64,
    /// tema (npr. "Hrana i piće")
    pub category: String,
    /// "Reč" ili "Rečenica"
    pub kind: String,
    pub sr: String,
    pub de: String,
    pub pronunciation: String,
    /// "Nemački primer. — Srpski prevod."
    pub example: String,
    /// A1, A2, B1, B2, X (dodatno) ili ""
    pub level: String,
    /// imenica / glagol / pridev / ostalo / rečenica
    pub pos: String,
    /// množina (imenice) ili Perfekt (glagoli)
    pub forms: String,
}

impl Entry {
    pub fn is_sentence(&self) -> bool {
        self.kind == TYPE_SENTENCE
    }

    pub fn forms_label(&self) -> String {
        if self.forms.is_empty() {
            return String::new();
        }
        match self.pos.as_str() {
            "glagol" => format!("Perfekt: {}", self.forms),
            "imenica" => {
                if self.forms.starts_with('(') {
                    "samo množina".into()
                } else if self.forms == "—" {
                    "nema množinu".into()
                } else {
                    format!("Množina: {}", self.forms)
                }
            }
            _ => self.forms.clone(),
        }
    }

    /// ("Nemački primer.", "Srpski prevod.")
    pub fn example_parts(&self) -> (&str, &str) {
        match self.example.split_once(" — ") {
            Some((de, sr)) => (de.trim(), sr.trim()),
            None => (self.example.trim(), ""),
        }
    }
}

pub const LEVEL_ORDER: [&str; 6] = ["A1", "A2", "B1", "B2", "X", ""];

pub const TOPIC_ORDER: [&str; 22] = [
    "Pozdravi i osnovni izrazi", "Lični podaci i porodica", "Brojevi, vreme i datumi",
    "Hrana i piće", "Kupovina i novac", "Stan i kuća", "Grad, saobraćaj i putovanje",
    "Posao i zanimanja", "Škola i učenje", "Telo i zdravlje", "Odeća",
    "Slobodno vreme i sport", "Priroda, životinje i vreme",
    "Mediji, tehnika i komunikacija", "Osećanja i osobine",
    "Društvo, država i praznici", "Usluge i ustanove",
    "Mišljenje i apstraktni pojmovi", "Opšti glagoli", "Opšti pridevi", "Male reči",
    "Gramatika i pravila",
];

pub fn level_order(level: &str) -> usize {
    LEVEL_ORDER.iter().position(|l| *l == level).unwrap_or(LEVEL_ORDER.len())
}

pub fn topic_order(topic: &str) -> (u8, usize, Vec<(u8, u64, String)>) {
    match TOPIC_ORDER.iter().position(|t| *t == topic) {
        Some(i) => (0, i, Vec::new()),
        None => (1, 0, crate::text::natural_key(topic)),
    }
}

pub fn level_label(level: &str) -> &'static str {
    match level {
        "A1" => "🟢 A1 — početni",
        "A2" => "🟡 A2 — osnovni",
        "B1" => "🟠 B1 — srednji",
        "B2" => "🔴 B2 — viši srednji",
        "X" => "🔵 Dodatno (stručno)",
        _ => "⚪ Bez nivoa",
    }
}

pub fn level_short(level: &str) -> &str {
    match level {
        "X" => "Dod.",
        "" => "—",
        l => l,
    }
}
