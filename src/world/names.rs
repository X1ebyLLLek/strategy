use std::collections::HashSet;
use crate::rng::Rng;

const START: [&str; 40] = [
    "Ал", "Бор", "Вес", "Гра", "Дор", "Ер", "Зар", "Кал", "Лин", "Мор", "Нор", "Ост", "Пер", "Рад", "Сал", "Тар",
    "Ур", "Фер", "Хал", "Цер", "Чер", "Шал", "Яр", "Эст", "Бел", "Вал", "Гел", "Дан", "Кор", "Лаз", "Мир", "Нев",
    "Ор", "Пол", "Рус", "Сер", "Тор", "Вин", "Ард", "Иль",
];
const MID: [&str; 14] = ["ва", "ре", "ми", "до", "са", "ла", "ти", "но", "ве", "ра", "", "", "", ""];
const END: [&str; 12] = ["ния", "рия", "ланд", "стан", "бург", "ция", "ия", "а", "ея", "ард", "мар", "ость"];

/// A person's name: an initial and a surname.
pub fn person(rng: &mut Rng) -> String {
    const ENDS: [&str; 8] = ["ов", "ин", "ский", "ер", "ан", "ич", "ев", "ман"];
    const INITIALS: [&str; 16] = ["А", "Б", "В", "Г", "Д", "Е", "З", "И", "К", "Л", "М", "Н", "О", "П", "Р", "С"];
    loop {
        let (a, m, e) = (START[rng.below(START.len())], MID[rng.below(MID.len())], ENDS[rng.below(ENDS.len())]);
        let vowel = |c: Option<char>| c.is_some_and(|c| "аеёиоуыэюя".contains(c));
        let left = if m.is_empty() { a } else { m };
        if vowel(left.chars().last()) && vowel(e.chars().next()) {
            continue;
        }
        if !m.is_empty() && vowel(a.chars().last()) && vowel(m.chars().next()) {
            continue;
        }
        return format!("{}. {a}{m}{e}", INITIALS[rng.below(INITIALS.len())]);
    }
}

/// n distinct country names.
pub fn countries(rng: &mut Rng, n: usize) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::with_capacity(n);
    while out.len() < n {
        let (a, m, e) = (START[rng.below(START.len())], MID[rng.below(MID.len())], END[rng.below(END.len())]);
        // no two vowels at a joint ("Норраия")
        let vowel = |c: Option<char>| c.is_some_and(|c| "аеёиоуыэюя".contains(c));
        let left = if m.is_empty() { a } else { m };
        if vowel(left.chars().last()) && vowel(e.chars().next()) {
            continue;
        }
        if !m.is_empty() && vowel(a.chars().last()) && vowel(m.chars().next()) {
            continue;
        }
        let s = format!("{a}{m}{e}");
        if seen.insert(s.clone()) {
            out.push(s);
        }
    }
    out
}

/// A town's name, not one of `taken` (it is added there).
pub fn place(rng: &mut Rng, taken: &mut HashSet<String>) -> String {
    const TOWN: [&str; 18] = ["град", "бург", "ск", "ов", "поль", "ин", "ец", "ево", "берг", "мир", "дар", "горск", "вель", "ань", "ор", "иц", "ино", "штадт"];
    loop {
        let (a, m, e) = (START[rng.below(START.len())], MID[rng.below(MID.len())], TOWN[rng.below(TOWN.len())]);
        let vowel = |c: Option<char>| c.is_some_and(|c| "аеёиоуыэюя".contains(c));
        let left = if m.is_empty() { a } else { m };
        if vowel(left.chars().last()) && vowel(e.chars().next()) {
            continue;
        }
        if !m.is_empty() && vowel(a.chars().last()) && vowel(m.chars().next()) {
            continue;
        }
        let s = format!("{a}{m}{e}");
        if taken.insert(s.clone()) {
            return s;
        }
    }
}

/// The word after a number: one, a few, many ("1 год", "3 года", "11 лет").
pub fn plural<'a>(n: usize, one: &'a str, few: &'a str, many: &'a str) -> &'a str {
    match (n % 10, n % 100) {
        (_, 11..=14) => many,
        (1, _) => one,
        (2..=4, _) => few,
        _ => many,
    }
}

/// The gender of a name as the words agreeing with it see it (0 — he, 1 — she, 2 — it, 3 — they),
/// by the ending of its head word: the first, unless that is an adjective («Военная диктатура»,
/// «Вольница вождей», «Фернобург», «Самодержавие»).
fn gender(subject: &str) -> u8 {
    let words: Vec<String> = subject.split_whitespace().map(|w| w.to_lowercase()).collect();
    let adjective = |w: &str| ["ая", "ое", "ый", "ий", "ой", "ые"].iter().any(|e| w.ends_with(e));
    let head = match words.as_slice() {
        [] => return 1,
        [first, .., last] if adjective(first) => last.as_str(),
        [first, ..] => first.as_str(),
    };
    if head.ends_with('а') || head.ends_with('я') || head.ends_with("ость") {
        1
    } else if head.ends_with('о') || head.ends_with('е') {
        2
    } else if head.ends_with('ы') || head.ends_with('и') {
        3
    } else {
        0
    }
}

/// Words given as they agree with a feminine name (`капитулировала`, `не пришла`, `связанная`) put
/// in the gender of `subject`: «Винмирия капитулировала», «Фернобург капитулировал», «самодержавие
/// пало». (A country's name is of any gender; the texts had them all feminine.)
pub fn agree(subject: &str, fem: &str) -> String {
    let g = gender(subject);
    if g == 1 {
        return fem.to_string();
    }
    // feminine ending → he, it, they
    const FORMS: [(&str, [&str; 3]); 6] = [
        ("шла", ["шёл", "шло", "шли"]),
        ("лась", ["лся", "лось", "лись"]),
        ("гла", ["г", "гло", "гли"]),
        ("ла", ["л", "ло", "ли"]),
        ("ная", ["ный", "ное", "ные"]),
        ("ая", ["ый", "ое", "ые"]),
    ];
    let k = match g {
        0 => 0,
        2 => 1,
        _ => 2,
    };
    fem.split(' ')
        .map(|w| FORMS.iter().find_map(|(end, to)| w.strip_suffix(end).map(|stem| format!("{stem}{}", to[k]))).unwrap_or_else(|| w.to_string()))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::agree;

    #[test]
    fn words_agree_with_names() {
        assert_eq!(agree("Винмирия", "капитулировала"), "капитулировала");
        assert_eq!(agree("Фернобург", "капитулировала"), "капитулировал");
        assert_eq!(agree("Русремар", "не пришла"), "не пришёл");
        assert_eq!(agree("Эствеланд", "отказалась"), "отказался");
        assert_eq!(agree("Невсастан", "отвергла"), "отверг");
        assert_eq!(agree("самодержавие", "пала"), "пало");
        assert_eq!(agree("Военная диктатура", "пала"), "пала");
        assert_eq!(agree("Партийное государство", "пала"), "пало");
        assert_eq!(agree("Фернобург", "связанная"), "связанный");
    }
}
