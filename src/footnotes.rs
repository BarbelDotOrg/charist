use crate::bibles::Verse;

/// A single in-text marker (note or cross-ref) with its byte offset into the
/// verse's plain text, ready to be numbered in reading order.
pub enum FootnoteMarker {
    Note(String),
    CrossRef(Vec<String>),
}

/// The payload carried by a clicked superscript, used both as the rich-text
/// `Link` type and as the data shown in the popup.
#[derive(Debug, Clone)]
pub enum FootnoteLink {
    Note { number: usize, text: String },
    CrossRef { number: usize, refs: Vec<String> },
}

pub fn collect_footnote_markers(verse: &Verse) -> Vec<(usize, FootnoteMarker)> {
    let mut items: Vec<(usize, FootnoteMarker)> = Vec::new();

    for note in &verse.1 {
        items.push((note.offset, FootnoteMarker::Note(note.text.clone())));
    }
    for cross_ref in &verse.2 {
        items.push((
            cross_ref.offset,
            FootnoteMarker::CrossRef(cross_ref.references.clone()),
        ));
    }

    items.sort_by_key(|(offset, _)| *offset);
    items
}

/// If `idx` lands inside a word (non-whitespace on both sides), push it
/// forward to the end of that word so the marker doesn't split it. Offsets
/// that already fall on whitespace/punctuation are left alone.
pub fn snap_to_word_end(s: &str, idx: usize) -> usize {
    let idx = clamp_to_char_boundary(s, idx.min(s.len()));

    let before_is_word = s[..idx]
        .chars()
        .next_back()
        .is_some_and(|c| !c.is_whitespace());
    let after_is_word = s[idx..].chars().next().is_some_and(|c| !c.is_whitespace());

    if before_is_word && after_is_word {
        match s[idx..].find(char::is_whitespace) {
            Some(rel) => idx + rel,
            None => s.len(),
        }
    } else {
        idx
    }
}

pub fn clamp_to_char_boundary(s: &str, mut idx: usize) -> usize {
    if idx > s.len() {
        idx = s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

pub fn to_superscript(n: usize) -> String {
    n.to_string()
        .chars()
        .map(|c| match c {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            other => other,
        })
        .collect()
}

pub fn to_superscript_letter(n: usize) -> String {
    const LETTERS: [char; 26] = [
        'ᵃ', 'ᵇ', 'ᶜ', 'ᵈ', 'ᵉ', 'ᶠ', 'ᵍ', 'ʰ', 'ⁱ', 'ʲ', 'ᵏ', 'ˡ', 'ᵐ', 'ⁿ', 'ᵒ', 'ᵖ', 'q', 'ʳ',
        'ˢ', 'ᵗ', 'ᵘ', 'ᵛ', 'ʷ', 'ˣ', 'ʸ', 'ᶻ',
    ];
    if n >= 1 && n <= LETTERS.len() {
        LETTERS[n - 1].to_string()
    } else {
        format!("({n})")
    }
}
