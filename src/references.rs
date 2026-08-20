use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use crate::debug_utils::trace;

#[derive(Debug, PartialEq, Eq)]
pub struct Reference {
    pub(crate) osis_book: String,
    pub(crate) chapter: Option<usize>,
    pub(crate) start_verse: Option<usize>,
    pub(crate) end_verse: Option<usize>,
}

lazy_static! {
    static ref BOOK_MAP: HashMap<&'static str, &'static str> = {
        let mut m = HashMap::new();

        let mappings = [
            ("Genesis", "Gen", &["gen", "genesis"] as &[&str]),
            ("Exodus", "Exod", &["exod", "exodus", "ex"]),
            ("Leviticus", "Lev", &["lev", "leviticus", "le"]),
            ("Numbers", "Num", &["num", "numbers", "nu"]),
            ("Deuteronomy", "Deut", &["deut", "deuteronomy", "dt"]),
            ("Joshua", "Josh", &["josh", "joshua", "jos"]),
            ("Judges", "Judg", &["judg", "judges", "jdg"]),
            ("Ruth", "Ruth", &["ruth", "ru"]),
            ("1 Samuel", "1Sam", &["1samuel", "1sam", "1sa", "firstsamuel"]),
            ("2 Samuel", "2Sam", &["2samuel", "2sam", "2sa", "secondsamuel"]),
            ("1 Kings", "1Kgs", &["1kings", "1kgs", "1ki", "firstkings"]),
            ("2 Kings", "2Kgs", &["2kings", "2kgs", "2ki", "secondkings"]),
            ("1 Chronicles", "1Chr", &["1chronicles", "1chr", "1ch", "firstchronicles"]),
            ("2 Chronicles", "2Chr", &["2chronicles", "2chr", "2ch", "secondchronicles"]),
            ("Ezra", "Ezra", &["ezra", "ezr"]),
            ("Nehemiah", "Neh", &["nehemiah", "neh", "ne"]),
            ("Esther", "Esth", &["esther", "esth", "est"]),
            ("Job", "Job", &["job", "jb"]),
            ("Psalms", "Ps", &["psalms", "psalm", "ps"]),
            ("Proverbs", "Prov", &["proverbs", "prov", "pr"]),
            ("Ecclesiastes", "Eccl", &["ecclesiastes", "eccl", "ecc", "ec"]),
            ("Song of Solomon", "Song", &["songofsolomon", "songofsongs", "song", "sos", "canticles"]),
            ("Isaiah", "Isa", &["isaiah", "isa", "is"]),
            ("Jeremiah", "Jer", &["jeremiah", "jer", "je"]),
            ("Lamentations", "Lam", &["lamentations", "lam", "la"]),
            ("Ezekiel", "Ezek", &["ezekiel", "ezek", "eze"]),
            ("Daniel", "Dan", &["daniel", "dan", "da"]),
            ("Hosea", "Hos", &["hosea", "hos", "ho"]),
            ("Joel", "Joel", &["joel", "joe"]),
            ("Amos", "Amos", &["amos", "am"]),
            ("Obadiah", "Obad", &["obadiah", "obad", "ob"]),
            ("Jonah", "Jonah", &["jonah", "jon", "jnh"]),
            ("Micah", "Mic", &["micah", "mic", "mc"]),
            ("Nahum", "Nah", &["nahum", "nah", "na"]),
            ("Habakkuk", "Hab", &["habakkuk", "hab", "hb"]),
            ("Zephaniah", "Zeph", &["zephaniah", "zeph", "zp"]),
            ("Haggai", "Hag", &["haggai", "hag", "hg"]),
            ("Zechariah", "Zech", &["zechariah", "zech", "zc"]),
            ("Malachi", "Mal", &["malachi", "mal", "ml"]),
            ("Matthew", "Matt", &["matthew", "matt", "mt"]),
            ("Mark", "Mark", &["mark", "mrk", "mk"]),
            ("Luke", "Luke", &["luke", "luk", "lk"]),
            ("John", "John", &["john", "jhn", "jn"]),
            ("Acts", "Acts", &["acts", "act", "ac"]),
            ("Romans", "Rom", &["romans", "rom", "ro"]),
            ("1 Corinthians", "1Cor", &["1corinthians", "1cor", "1co", "firstcorinthians"]),
            ("2 Corinthians", "2Cor", &["2corinthians", "2cor", "2co", "secondcorinthians"]),
            ("Galatians", "Gal", &["galatians", "gal", "ga"]),
            ("Ephesians", "Eph", &["ephesians", "eph", "ep"]),
            ("Philippians", "Phil", &["philippians", "phil", "php", "pp"]),
            ("Colossians", "Col", &["colossians", "col"]),
            ("1 Thessalonians", "1Thess", &["1thessalonians", "1thess", "1th", "firstthessalonians"]),
            ("2 Thessalonians", "2Thess", &["2thessalonians", "2thess", "2th", "secondthessalonians"]),
            ("1 Timothy", "1Tim", &["1timothy", "1tim", "1ti", "firsttimothy"]),
            ("2 Timothy", "2Tim", &["2timothy", "2tim", "2ti", "secondtimothy"]),
            ("Titus", "Titus", &["titus", "tit", "ti"]),
            ("Philemon", "Phlm", &["philemon", "phlm", "phm"]),
            ("Hebrews", "Heb", &["hebrews", "heb"]),
            ("James", "Jas", &["james", "jas", "jm"]),
            ("1 Peter", "1Pet", &["1peter", "1pet", "1pe", "firstpeter"]),
            ("2 Peter", "2Pet", &["2peter", "2pet", "2pe", "secondpeter"]),
            ("1 John", "1John", &["1john", "1jn", "1j", "firstjohn"]),
            ("2 John", "2John", &["2john", "2jn", "2j", "secondjohn"]),
            ("3 John", "3John", &["3john", "3jn", "3j", "thirdjohn"]),
            ("Jude", "Jude", &["jude", "jud"]),
            ("Revelation", "Rev", &["revelation", "rev", "re", "apoc", "apocalypse"]),
        ];

        for (_, osis, aliases) in mappings {
            for alias in aliases {
                m.insert(*alias, osis);
            }
        }
        m
    };

    // Regex capturing book title prefix alongside optional chapter and verses
    // Dont ask me how this works i used claude :D
    static ref REF_REGEX: Regex = Regex::new(
        r"(?i)^\s*(?P<book>(?:\d\s*)?[a-zA-Z\s]+?)\.?(?:\s+|\.)?(?:(?P<chapter>\d+)(?:[:.](?P<start_v>\d+)(?:[\-–—](?P<end_v>\d+))?)?)?\s*$"
    ).unwrap();
}

// this functions takes a rather long time but i found that its not a problem
pub fn parse_reference(input: &str) -> Option<Reference> {
    trace("parse_reference", || {
        let caps = REF_REGEX.captures(input)?;

        let raw_book = caps.name("book")?.as_str();
        let normalized_key = raw_book
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect::<String>();

        let osis_book = (*BOOK_MAP.get(normalized_key.as_str())?).to_string();

        let chapter = caps
            .name("chapter")
            .and_then(|m| m.as_str().parse::<usize>().ok());
        let start_verse = caps
            .name("start_v")
            .and_then(|m| m.as_str().parse::<usize>().ok());
        let end_verse = caps
            .name("end_v")
            .and_then(|m| m.as_str().parse::<usize>().ok());

        Some(Reference {
            osis_book,
            chapter,
            start_verse,
            end_verse,
        })
    })
}
