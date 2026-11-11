//! Unicode-svesno poređenje i pretraga (isto ponašanje kao u Python verziji,
//! da bi ključevi napretka u bazi ostali kompatibilni).

/// Mala slova; `ß` -> `ss` (kao Python `str.casefold()`).
pub fn casefold(s: &str) -> String {
    s.to_lowercase().replace('ß', "ss")
}

fn strip_diacritic(c: char) -> Option<&'static str> {
    Some(match c {
        'š' => "s", 'č' | 'ć' | 'ç' => "c", 'ž' => "z",
        'ä' | 'á' | 'à' | 'â' | 'ã' | 'å' => "a",
        'ö' | 'ó' | 'ò' | 'ô' | 'õ' => "o",
        'ü' | 'ú' | 'ù' | 'û' => "u",
        'é' | 'è' | 'ê' | 'ë' => "e",
        'í' | 'ì' | 'î' | 'ï' => "i",
        'ñ' => "n",
        _ => return None,
    })
}

/// Za pretragu: mala slova, bez kvačica ('Käse' -> 'kase', 'đak' -> 'djak').
pub fn fold(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in casefold(s).chars() {
        if c == 'đ' {
            out.push_str("dj");
        } else if let Some(r) = strip_diacritic(c) {
            out.push_str(r);
        } else if !('\u{0300}'..='\u{036f}').contains(&c) {
            out.push(c);
        }
    }
    out
}

/// Normalizacija ukucanog odgovora: mala slova, ä->ae, ß->ss, bez kvačica,
/// bez interpunkcije i bez sadržaja u zagradama.
pub fn norm(text: &str) -> String {
    // izbaci (…)
    let mut s = String::with_capacity(text.len());
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '(' => { depth += 1; s.push(' '); }
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => s.push(c),
            _ => {}
        }
    }
    let mut out = String::with_capacity(s.len());
    for c in casefold(&s).chars() {
        match c {
            'ä' => out.push_str("ae"),
            'ö' => out.push_str("oe"),
            'ü' => out.push_str("ue"),
            'đ' => out.push_str("dj"),
            _ => {
                if let Some(r) = strip_diacritic(c) {
                    out.push_str(r);
                } else if c.is_alphanumeric() || c == '_' || c == '-' || c.is_whitespace() {
                    out.push(c);
                } else {
                    out.push(' ');
                }
            }
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Ključ za "prirodno" sortiranje: 'Lekcija 2' pre 'Lekcija 10'.
pub fn natural_key(s: &str) -> Vec<(u8, u64, String)> {
    let f = fold(s);
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut digits = false;
    for c in f.chars() {
        let d = c.is_ascii_digit();
        if d != digits && !cur.is_empty() {
            out.push(part(&cur, digits));
            cur.clear();
        }
        digits = d;
        cur.push(c);
    }
    if !cur.is_empty() {
        out.push(part(&cur, digits));
    }
    out
}

fn part(s: &str, digits: bool) -> (u8, u64, String) {
    if digits {
        (0, s.parse().unwrap_or(u64::MAX), String::new())
    } else {
        (1, 0, s.to_string())
    }
}

/// Levenštajnovo rastojanje.
pub fn distance(a: &str, b: &str) -> usize {
    if a == b {
        return 0;
    }
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            cur.push((prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + usize::from(ca != cb)));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn folding() {
        assert_eq!(casefold("Straße ŠUMA"), "strasse šuma");
        assert_eq!(fold("Käse Šta đak"), "kase sta djak");
        assert_eq!(norm("die Äpfel (mn.)!"), "die aepfel");
        assert_eq!(distance("tisch", "tish"), 1);
        assert!(natural_key("Lekcija 2") < natural_key("Lekcija 10"));
    }
}
