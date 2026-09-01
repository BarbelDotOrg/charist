#!/usr/bin/env python3
"""
sword_to_json.py

Export an installed SWORD Bible module to a CBOR document matching this
Rust layout (deserializable directly with serde_cbor / ciborium etc.):

    struct BibleData {
        meta: Meta,                      // { module, generated_at }
        book_order: Vec<String>,         // OSIS book keys, canonical order
        books: HashMap<String, Book>,    // keyed by OSIS book key
        language: String,
        description: String,
    }

    struct Book {
        name: String,                    // full English name, e.g. "Genesis"
        osis_name: String,               // e.g. "Gen"
        abbreviation: String,            // e.g. "Gen"
        chapters: Vec<Vec<Verse>>,       // chapters[0] = chapter 1, etc.
    }

    // tuple struct -> JSON array of length 3
    struct Verse(String, Vec<Note>, Vec<CrossRef>);

    struct Note        { offset: usize, text: String }
    struct CrossRef    { offset: usize, references: Vec<String> }

REQUIREMENTS
------------
This script uses the official SWORD C++ library's SWIG-generated Python
bindings (the `Sword` module), NOT the pure-python `pysword` package. On
Debian/Ubuntu:

    sudo apt install python3-sword libsword-dev

Also requires `lxml` for lenient parsing of the raw OSIS markup, and
`cbor2` for writing the output:

    pip install lxml cbor2

USAGE
-----
    python3 sword_to_json.py --list
    python3 sword_to_json.py engKJV2006eb -o kjv.cbor
    python3 sword_to_json.py engKJV2006eb -o kjv.cbor --description "King James Version (1769)"

Module names are whatever your installed .conf files call them -- these
are frequently NOT the short names you might expect (e.g. Debian's
sword-text-kjv package installs as "engKJV2006eb", not "KJV"). Always
check `--list` first.

By default, `description` in the output is taken from the module's own
config ("Description" in its .conf file). Pass --description to
override it with your own text instead.

If no modules are found at all, your SWORD data directory (containing
mods.d/ and modules/) may not be somewhere SWMgr() looks by default
(env $SWORD_PATH, ~/.sword, /etc/sword.conf-listed paths, system dirs).
Use --sword-path to point at it directly.

NOTES / CAVEATS
----------------
* This targets modules whose SourceType is OSIS (the vast majority of
  modern SWORD modules). Modules using GBF/ThML/plain markup fall back
  to plain rendered text with empty notes/cross-references (a warning
  is printed).
* Cross-reference notes (<note type="crossReference">) are parsed by
  collecting their <reference> children, preferring an osisRef
  attribute when present and otherwise falling back to the reference's
  plain text, split on ';' (this is what real-world modules like the
  WEB actually do -- osisRef is frequently absent). A <reference
  type="annotateRef"> child (a self-locator like "1:1 ") is always
  skipped, for both footnotes and cross-references.
* offsets are counted in Python character length into the plain verse
  text, at the point the note/cross-reference was removed from the
  text (so notes don't count toward the length of the surrounding
  text). Removing a note leaves a single space in its place so words
  on either side of it don't get fused together (e.g. "God<note>...
  </note>created" would otherwise read "Godcreated").
"""

import argparse
import datetime
import re
import sys

try:
    from lxml import etree
except ImportError:
    print("This script requires lxml: pip install lxml", file=sys.stderr)
    sys.exit(1)

try:
    import cbor2
except ImportError:
    print("This script requires cbor2: pip install cbor2", file=sys.stderr)
    sys.exit(1)

try:
    import Sword
except ImportError:
    print(
        "Could not import the 'Sword' python module (the SWIG bindings for "
        "libsword). Install your distro's sword bindings package, e.g. "
        "'python3-sword' on Debian/Ubuntu, or build them from the SWORD "
        "source tree.",
        file=sys.stderr,
    )
    sys.exit(1)


def local_tag(tag):
    """Strip an XML namespace off an lxml tag, e.g. '{ns}note' -> 'note'."""
    if isinstance(tag, str) and tag.startswith("{"):
        return tag.split("}", 1)[1]
    return tag


def make_mgr(sword_path=None):
    """Build an SWMgr. `sword_path`, if given, is passed as the config/data
    path so modules outside the default search locations (env SWORD_PATH,
    ~/.sword, /etc/sword.conf, system dirs) can still be found.

    Configured with a plain-text MarkupFilterMgr so that renderText() (used
    for the non-OSIS fallback path) returns tag-free plain text instead of
    raw markup. `thisown = False` is required here: without it, both the
    SWMgr and Python's garbage collector try to delete the underlying C++
    MarkupFilterMgr, which segfaults at interpreter exit.
    """
    markup = Sword.MarkupFilterMgr(Sword.FMT_PLAIN)
    markup.thisown = False
    if sword_path:
        # SWMgr(iConfigPath, autoload, filterMgr, multiMod, augmentHome)
        return Sword.SWMgr(sword_path, True, markup, False, True)
    # SWMgr(filterMgr, multiMod) -- passing None explicitly for iConfigPath
    # here instead finds zero modules, so this overload must be used bare.
    return Sword.SWMgr(markup)


def module_names(mgr):
    """List installed module names. mgr.getModules() returns a ModuleMap
    whose keys are Sword.SWBuf objects, not python strings -- str() each
    one to get a real, sortable, printable name."""
    return sorted(str(name) for name in mgr.getModules().keys())


def get_module(mgr, module_name):
    module = mgr.getModule(module_name)
    if module is None:
        available = module_names(mgr)
        if available:
            hint = "Installed modules: " + ", ".join(available)
        else:
            hint = (
                "No modules were found at all. SWMgr() looks in $SWORD_PATH, "
                "~/.sword, /etc/sword.conf-listed paths, and system dirs "
                "(e.g. /usr/share/sword). If your module lives somewhere "
                "else, pass --sword-path /path/to/your/sword/dir (the "
                "directory that contains mods.d/ and modules/)."
            )
        raise SystemExit(f"Module '{module_name}' not found. {hint}")
    return module


def is_annotate_ref(elem):
    return local_tag(elem.tag) == "reference" and (
        elem.get("type") or ""
    ).lower() == "annotateref"


def parse_crossref_note(note_elem):
    """Extract a list of reference strings from a crossReference note.
    Real-world modules (e.g. WEB) often don't set osisRef at all and just
    put plain, semicolon-separated reference text inside <reference>."""
    refs = []
    for el in note_elem.iter():
        if el is note_elem or local_tag(el.tag) != "reference":
            continue
        if is_annotate_ref(el):
            continue
        osis_ref = el.get("osisRef")
        if osis_ref:
            refs.append(osis_ref)
        else:
            text = "".join(el.itertext()).strip()
            if text:
                refs.extend(p.strip() for p in text.split(";") if p.strip())
    return refs


def note_plain_text(note_elem):
    """Plain text of a footnote, skipping any <reference type="annotateRef">
    self-locator (e.g. "1:1 ") that isn't part of the actual note content."""
    parts = []
    if note_elem.text:
        parts.append(note_elem.text)
    for child in note_elem:
        if is_annotate_ref(child):
            if child.tail:
                parts.append(child.tail)
            continue
        parts.append("".join(child.itertext()))
        if child.tail:
            parts.append(child.tail)
    return " ".join("".join(parts).split())


def process_node(elem, buf, notes, crossrefs):
    """Recursively walk an OSIS verse fragment, building plain text and
    collecting notes/cross-references with character offsets into that
    plain text."""
    if elem.text:
        buf.append(elem.text)

    for child in elem:
        tag = local_tag(child.tag)

        if tag == "note":
            offset = sum(len(s) for s in buf)
            note_type = (child.get("type") or "").lower()
            if note_type == "crossreference":
                refs = parse_crossref_note(child)
                crossrefs.append({"offset": offset, "references": refs})
            else:
                text = note_plain_text(child)
                if text:
                    notes.append({"offset": offset, "text": text})
            # The note itself is removed from the reading text; leave a
            # single space so words on either side don't get fused
            # together (raw OSIS often has no whitespace around <note>).
            buf.append(" ")
            if child.tail:
                buf.append(child.tail)
            continue

        if tag == "title":
            # Section/canonical titles aren't part of verse content proper.
            if child.tail:
                buf.append(child.tail)
            continue

        # Recurse into everything else (w, transChange, divineName, seg,
        # foreign, l, lg, q, div milestones, etc.) treating their text as
        # part of the verse.
        process_node(child, buf, notes, crossrefs)
        if child.tail:
            buf.append(child.tail)


def extract_verse(raw_osis):
    """Parse a raw OSIS verse fragment into (text, notes, crossrefs)."""
    wrapped = f"<verse>{raw_osis}</verse>"
    parser = etree.XMLParser(recover=True, resolve_entities=False)
    try:
        root = etree.fromstring(wrapped.encode("utf-8"), parser=parser)
    except Exception:
        # Last-ditch fallback: strip all tags crudely.
        text = re.sub(r"<[^>]+>", "", raw_osis)
        return " ".join(text.split()), [], []

    if root is None:
        return "", [], []

    buf, notes, crossrefs = [], [], []
    process_node(root, buf, notes, crossrefs)
    text = " ".join("".join(buf).split())
    return text, notes, crossrefs


def module_uses_osis(module):
    try:
        src_type = module.getConfigEntry("SourceType") or ""
    except Exception:
        src_type = ""
    return str(src_type).strip().upper() == "OSIS"


def export_module(module_name, sword_path=None, description=None):
    mgr = make_mgr(sword_path)
    module = get_module(mgr, module_name)

    is_osis = module_uses_osis(module)
    if not is_osis:
        print(
            f"Warning: module '{module_name}' SourceType is not OSIS; "
            "falling back to plain rendered text with no "
            "notes/cross-references.",
            file=sys.stderr,
        )

    try:
        language = module.getLanguage() or ""
    except Exception:
        language = ""

    if description is None:
        # No --description given: fall back to the module's own metadata.
        try:
            description = module.getConfigEntry("Description") or ""
        except Exception:
            description = ""

    # A fresh VerseKey defaults to Gen 1:1. setPersist(True) makes the
    # module track/own this key object across increments; AutoNormalize
    # off avoids surprises when landing on invalid chapter/verse numbers.
    key = Sword.VerseKey()
    key.setPersist(True)
    key.setAutoNormalize(False)
    module.setKey(key)

    books = {}
    book_order = []
    current_osis_book = None
    current_chapter_num = None

    while True:
        vkey = Sword.VerseKey.castTo(module.getKey())
        osis_book = vkey.getOSISBookName()
        book_name = vkey.getBookName()
        abbreviation = vkey.getBookAbbrev()
        chapter_num = vkey.getChapter()

        if osis_book != current_osis_book:
            current_osis_book = osis_book
            current_chapter_num = None
            if osis_book not in books:
                book_order.append(osis_book)
                books[osis_book] = {
                    "name": book_name,
                    "osis_name": osis_book,
                    "abbreviation": abbreviation,
                    "chapters": [],
                }

        if chapter_num != current_chapter_num:
            current_chapter_num = chapter_num
            books[osis_book]["chapters"].append([])

        if is_osis:
            raw = module.getRawEntry()
            text, notes, crossrefs = extract_verse(raw)
        else:
            text = str(module.renderText() or "").strip()
            notes, crossrefs = [], []

        verse_tuple = [text, notes, crossrefs]
        books[osis_book]["chapters"][-1].append(verse_tuple)

        module.increment(1)
        # popError() returns a *char*, and '\x00' (no error) is still a
        # truthy non-empty string in Python -- must compare by ordinal,
        # not truthiness, or iteration stops after the very first verse.
        if ord(module.popError()) != 0:
            break

    return {
        "meta": {
            "module": module_name,
            "generated_at": datetime.datetime.now(datetime.timezone.utc)
            .isoformat()
            .replace("+00:00", "Z"),
        },
        "book_order": book_order,
        "books": books,
        "language": language,
        "description": description,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("module", nargs="?", help="SWORD module name, e.g. engKJV2006eb")
    parser.add_argument("-o", "--output", help="Output CBOR file path")
    parser.add_argument(
        "--description",
        help=(
            "Description string to embed in the output. Defaults to the "
            "module's own 'Description' config entry if omitted."
        ),
    )
    parser.add_argument(
        "--list", action="store_true", help="List installed modules and exit"
    )
    parser.add_argument(
        "--sword-path",
        help=(
            "Explicit SWORD data directory (containing mods.d/ and modules/), "
            "for installs SWMgr() wouldn't find by default."
        ),
    )
    args = parser.parse_args()

    if args.list:
        mgr = make_mgr(args.sword_path)
        names = module_names(mgr)
        if not names:
            print(
                "No modules found. Try --sword-path /path/to/your/sword/dir, "
                "or check $SWORD_PATH / ~/.sword.",
                file=sys.stderr,
            )
        for name in names:
            print(name)
        return

    if not args.module:
        parser.error("module name is required unless --list is given")

    data = export_module(
        args.module, sword_path=args.sword_path, description=args.description
    )

    out_path = args.output or f"{args.module}.cbor"
    with open(out_path, "wb") as f:
        cbor2.dump(data, f)

    print(f"Wrote {out_path}", file=sys.stderr)


if __name__ == "__main__":
    main()