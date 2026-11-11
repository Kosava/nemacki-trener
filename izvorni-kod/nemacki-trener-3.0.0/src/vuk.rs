//! Automatski izgovor nemačkog „po Vuku" (srpska latinica).
//! Pravila su ista kao u Python verziji (utils/vuk.py).

const EXCEPTIONS: &[(&str, &str)] = &[
    ("chef", "šef"), ("chance", "šanse"), ("chaos", "kaos"), ("chor", "kor"),
    ("charakter", "karakter"), ("christ", "krist"), ("chemie", "hemi"),
    ("computer", "kompjuter"), ("handy", "hendi"), ("job", "džob"), ("team", "tim"),
    ("e-mail", "imejl"), ("email", "imejl"), ("jeans", "džins"), ("baby", "bebi"),
    ("manager", "menedžer"), ("show", "šou"), ("party", "parti"), ("toilette", "toalete"),
    ("restaurant", "restoran"), ("orange", "oranže"), ("garage", "garaže"),
    ("etage", "etaže"), ("journalist", "žurnalist"), ("ingenieur", "inženjer"),
    ("friseur", "frizer"), ("trainer", "trener"), ("portemonnaie", "portmone"),
    ("café", "kafe"), ("cafe", "kafe"), ("museum", "muzeum"), ("familie", "familije"),
    ("linie", "linije"), ("ferien", "ferijen"), ("bus", "bus"), ("pizza", "pica"),
    ("ticket", "tiket"), ("laptop", "leptop"), ("internet", "internet"),
    ("interview", "intervju"), ("user", "juzer"), ("online", "onlajn"),
    ("sweatshirt", "svetšert"), ("t-shirt", "tišert"), ("camping", "kemping"),
    ("ski", "ši"), ("skifahren", "šifaren"), ("chips", "čips"), ("cola", "kola"),
    ("der", "der"), ("die", "di"), ("das", "das"), ("des", "des"), ("dem", "dem"),
    ("den", "den"), ("sie", "zi"), ("sieh", "zi"), ("ihr", "ir"), ("ihm", "im"),
    ("ihn", "in"), ("ihnen", "inen"), ("ich", "ih"), ("mich", "mih"), ("dich", "dih"),
    ("sich", "zih"), ("nicht", "niht"), ("auch", "auh"), ("noch", "noh"),
    ("doch", "doh"), ("nach", "nah"), ("jetzt", "jec"), ("hier", "hir"), ("vier", "fir"),
    ("viel", "fil"), ("viele", "file"), ("vielen", "filen"), ("vielleicht", "filajht"),
    ("von", "fon"), ("vor", "for"), ("voll", "fol"), ("vom", "fom"), ("vater", "fater"),
    ("vogel", "fogel"), ("volk", "folk"), ("es", "es"), ("uhr", "ur"),
];

const V_AS_F: &[&str] = &[
    "vor", "ver", "voll", "viel", "vier", "vat", "vög", "vog", "volk", "von", "vers",
    "vieh", "vetter", "vorn",
];

fn exception(low: &str) -> Option<&'static str> {
    EXCEPTIONS.iter().find(|(k, _)| *k == low).map(|(_, v)| *v)
}

fn is_vowel(c: char) -> bool {
    "aeiouäöüy".contains(c)
}

fn word(w: &str) -> String {
    let low = w.to_lowercase();
    if let Some(e) = exception(&low) {
        return e.to_string();
    }
    // Privremene oznake: \x01 = h od 'ch', \x02 = c od 'z/tz', \x03 = s od 'ß'
    let mut s = low.replace("entsch", "ent\u{4}");
    // (?<=[aeiouäöü])tsch|^tsch -> č
    {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len());
        let mut i = 0;
        while i < chars.len() {
            let is_tsch = i + 4 <= chars.len() && chars[i..i + 4] == ['t', 's', 'c', 'h'];
            if is_tsch && (i == 0 || "aeiouäöü".contains(chars[i - 1])) {
                out.push('č');
                i += 4;
            } else {
                out.push(chars[i]);
                i += 1;
            }
        }
        s = out;
    }
    s = s.replace("sch", "š").replace('\u{4}', "š").replace("chs", "ks");
    if let Some(rest) = s.strip_prefix("ch") {
        s = format!("š{rest}");
    }
    s = s.replace("ch", "\u{1}").replace("tion", "\u{2}ion").replace('ß', "\u{3}");
    for (a, b) in [
        ("ck", "k"), ("dt", "t"), ("th", "t"), ("ph", "f"), ("qu", "kv"),
        ("tz", "\u{2}"), ("zz", "\u{2}"), ("z", "\u{2}"),
        ("ei", "aj"), ("ey", "aj"), ("ai", "aj"), ("äu", "oj"), ("eu", "oj"),
    ] {
        s = s.replace(a, b);
    }
    s = s.replace("ie", "i");
    for a in ["aa", "ee", "oo"] {
        s = s.replace(a, &a[..1]);
    }
    if let Some(rest) = s.strip_prefix("st") {
        s = format!("št{rest}");
    } else if let Some(rest) = s.strip_prefix("sp") {
        s = format!("šp{rest}");
    }
    if s.starts_with('v') && V_AS_F.iter().any(|p| low.starts_with(p)) {
        s = format!("f{}", &s[1..]);
    }
    // (?<=.)v(?=or) -> f
    {
        let chars: Vec<char> = s.chars().collect();
        let mut out = String::with_capacity(s.len());
        for (i, &c) in chars.iter().enumerate() {
            if c == 'v' && i > 0 && chars.get(i + 1) == Some(&'o') && chars.get(i + 2) == Some(&'r') {
                out.push('f');
            } else {
                out.push(c);
            }
        }
        s = out;
    }

    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    for (i, &ch) in chars.iter().enumerate() {
        let prev = if i > 0 { Some(chars[i - 1]) } else { None };
        let next = chars.get(i + 1).copied();
        if ch == 'h' && prev.is_some_and(is_vowel) {
            continue; // rastezno h: gehen, Fahrrad
        }
        if Some(ch) == prev && !is_vowel(ch) {
            continue; // dupli suglasnik: Mutter
        }
        if ch == 's'
            && next.is_some_and(is_vowel)
            && (i == 0 || prev.is_some_and(|p| is_vowel(p) || "jlnmr".contains(p)))
        {
            out.push('z'); // Sonne, Hose
            continue;
        }
        match ch {
            'w' => out.push('v'),
            'ä' | 'ö' => out.push('e'),
            'ü' | 'y' => out.push('i'),
            'x' => out.push_str("ks"),
            'c' => out.push('k'),
            _ => out.push(ch),
        }
    }
    let mut s = out.replace('\u{1}', "h").replace('\u{2}', "c").replace('\u{3}', "s");
    // obezvučenje na kraju reči
    if s.ends_with("ig") {
        s.truncate(s.len() - 2);
        s.push_str("ih");
    }
    if !s.ends_with("ng") && s.chars().count() >= 2 && s.ends_with('g') {
        s.pop();
        s.push('k');
    }
    if s.ends_with('d') {
        s.pop();
        s.push('t');
    }
    if s.ends_with('b') {
        s.pop();
        s.push('p');
    }
    s
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphabetic() || "ÄÖÜäöüßé-".contains(c)
}

/// Prevodi nemački tekst (reč, frazu ili rečenicu) u izgovor po Vuku.
pub fn to_vuk(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut token = String::new();
    let flush = |token: &mut String, out: &mut String| {
        if token.is_empty() {
            return;
        }
        if token.contains('-') && exception(&token.to_lowercase()).is_none() {
            let parts: Vec<String> = token.split('-').map(word).collect();
            out.push_str(&parts.join("-"));
        } else {
            out.push_str(&word(token));
        }
        token.clear();
    };
    for c in text.chars() {
        if is_word_char(c) {
            token.push(c);
        } else {
            flush(&mut token, &mut out);
            out.push(c);
        }
    }
    flush(&mut token, &mut out);
    let cleaned: String = out
        .replace('\u{1}', "h")
        .replace('\u{2}', "c")
        .replace('\u{3}', "s")
        .chars()
        .filter(|c| !".!?;:„“”\"".contains(*c))
        .collect();
    cleaned
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(|c| c == ' ' || c == ',')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::to_vuk;

    #[test]
    fn basics() {
        assert_eq!(to_vuk("der Schmetterling"), "der šmeterling");
        assert_eq!(to_vuk("die Straße"), "di štrase");
        assert_eq!(to_vuk("gehen"), "geen");
        assert_eq!(to_vuk("die Zeitung"), "di cajtung");
        assert_eq!(to_vuk("kochen"), "kohen");
    }

    /// Poređenje sa Python verzijom na celom rečniku (ako postoji fixture).
    #[test]
    fn matches_python() {
        let Ok(path) = std::env::var("VUK_FIXTURE") else { return };
        let data = std::fs::read_to_string(path).unwrap();
        let mut bad = 0;
        for line in data.lines() {
            let (de, exp) = line.split_once('\t').unwrap();
            let got = to_vuk(de);
            if got != exp {
                bad += 1;
                if bad <= 15 {
                    eprintln!("{de:?}: rust={got:?} python={exp:?}");
                }
            }
        }
        assert_eq!(bad, 0, "razlika u izgovoru");
    }
}
