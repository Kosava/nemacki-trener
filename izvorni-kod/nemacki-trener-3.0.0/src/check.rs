//! Provera ukucanog odgovora i pravila za rod imenica (der/die/das).

use crate::text::{distance, norm};

pub const ARTICLES: [&str; 3] = ["der", "die", "das"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Typo,
    Article,
    Wrong,
}

#[derive(Debug, Clone)]
pub struct CheckResult {
    pub status: Status,
    pub message: String,
}

impl CheckResult {
    pub fn new(status: Status, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }
    pub fn correct(&self) -> bool {
        matches!(self.status, Status::Ok | Status::Typo)
    }
}

/// 'pomoći / pomagati' ili 'pomoći; pomagati' -> više prihvatljivih odgovora.
fn alternatives(expected: &str) -> Vec<&str> {
    let parts: Vec<&str> = expected
        .split(['/', ';'])
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if parts.is_empty() { vec![expected] } else { parts }
}

fn split_article(n: &str) -> (Option<&str>, &str) {
    if let Some((head, rest)) = n.split_once(' ') {
        if ARTICLES.contains(&head) && !rest.is_empty() {
            return (Some(head), rest);
        }
    }
    (None, n)
}

pub fn check_answer(user: &str, expected: &str) -> CheckResult {
    let u = norm(user);
    if u.is_empty() {
        return CheckResult::new(Status::Wrong, "Nisi ništa ukucao.");
    }
    let mut best: Option<CheckResult> = None;
    for alt in alternatives(expected) {
        let e = norm(alt);
        if u == e {
            return CheckResult::new(Status::Ok, "Tačno!");
        }
        let (ua, uw) = split_article(&u);
        let (ea, ew) = split_article(&e);
        if let Some(ea) = ea {
            if uw == ew && ua != Some(ea) {
                let msg = match ua {
                    None => format!("Reč je tačna, ali fali član — {ea}."),
                    Some(ua) => format!("Pogrešan član: {ua} → tačno je {ea}."),
                };
                if best.is_none() {
                    best = Some(CheckResult::new(Status::Article, msg));
                }
                continue;
            }
        }
        let len = e.chars().count();
        let allowed = if len < 10 { 1 } else { 2 };
        if len >= 4 && distance(&u, &e) <= allowed {
            best = Some(CheckResult::new(
                Status::Typo,
                "Skoro tačno — greška u kucanju, računa se kao tačno.",
            ));
        }
    }
    best.unwrap_or_else(|| CheckResult::new(Status::Wrong, "Netačno."))
}

/// 'die Wohnung' -> ("die", "Wohnung"); za fraze i nabrajanja vraća None.
pub fn split_noun(de: &str) -> Option<(&str, &str)> {
    let (art, noun) = de.trim().split_once(' ')?;
    if !ARTICLES.contains(&art) {
        return None;
    }
    let first = noun.chars().next()?;
    if !(first.is_ascii_uppercase() || "ÄÖÜ".contains(first)) {
        return None;
    }
    noun.chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        .then_some((art, noun))
}

const GENDER_RULES: &[(&str, &str, &str)] = &[
    ("chen", "das", "umanjenice na -chen su uvek das"),
    ("lein", "das", "umanjenice na -lein su uvek das"),
    ("ung", "die", "imenice na -ung su uvek die"),
    ("heit", "die", "imenice na -heit su uvek die"),
    ("keit", "die", "imenice na -keit su uvek die"),
    ("schaft", "die", "imenice na -schaft su uvek die"),
    ("tion", "die", "imenice na -tion su uvek die"),
    ("sion", "die", "imenice na -sion su uvek die"),
    ("tät", "die", "imenice na -tät su uvek die"),
    ("ei", "die", "imenice na -ei su skoro uvek die"),
    ("ie", "die", "imenice na -ie su skoro uvek die"),
    ("ik", "die", "imenice na -ik su obično die"),
    ("ur", "die", "imenice na -ur su obično die"),
    ("ismus", "der", "imenice na -ismus su uvek der"),
    ("ling", "der", "imenice na -ling su uvek der"),
    ("ment", "das", "imenice na -ment su obično das"),
    ("um", "das", "imenice na -um su obično das"),
    ("ent", "der", "imenice na -ent (osobe) su obično der"),
    ("ist", "der", "imenice na -ist (osobe) su obično der"),
    ("or", "der", "imenice na -or su obično der"),
];

/// Kratko pravilo koje objašnjava rod, ako postoji i ako se slaže.
pub fn gender_hint(noun: &str, article: &str) -> String {
    let low = noun.to_lowercase();
    for (suffix, art, text) in GENDER_RULES {
        if low.ends_with(suffix) && *art == article {
            return format!("Pravilo: {text}.");
        }
    }
    if article == "die" && low.ends_with('e') {
        return "Pravilo: većina imenica na -e je die.".into();
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers() {
        assert_eq!(check_answer("der Tisch", "der Tisch").status, Status::Ok);
        assert_eq!(check_answer("Tisch", "der Tisch").status, Status::Article);
        assert_eq!(check_answer("die Tisch", "der Tisch").status, Status::Article);
        assert_eq!(check_answer("der Tish", "der Tisch").status, Status::Typo);
        assert_eq!(check_answer("Stuhl", "der Tisch").status, Status::Wrong);
        assert_eq!(check_answer("pomagati", "pomoći / pomagati").status, Status::Ok);
        assert_eq!(check_answer("die Aepfel", "die Äpfel").status, Status::Ok);
        assert_eq!(check_answer("sto", "sto (nameštaj)").status, Status::Ok);
    }

    #[test]
    fn nouns() {
        assert_eq!(split_noun("die Wohnung"), Some(("die", "Wohnung")));
        assert_eq!(split_noun("guten Tag"), None);
        assert_eq!(split_noun("der Kaffee mit Milch"), None);
        assert!(gender_hint("Wohnung", "die").contains("-ung"));
    }
}
