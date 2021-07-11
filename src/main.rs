// Copyright (c) 2021, Andrey Dubovik <andrei@dubovik.eu>

// Standard library
use std::collections::{HashSet, HashMap, BTreeMap};
use std::io::{self, BufRead};
use std::iter;

// Crates
use lazy_static::lazy_static;
use phf::{phf_set, phf_map};
use quick_xml::Reader;
use quick_xml::events::Event;
use regex::Regex;
use serde::Serialize;
use serde::ser::{Serializer, SerializeSeq};
use serde_json;

// Local modules
mod partitioner;
mod template;
use partitioner::Partitioner;


// Create an iterator over wiktionary pages
// (assumes the wiktionary dump is well-formatted, which it is)
fn pages(reader: impl BufRead) -> impl Iterator<Item = (String, String)> {
    let mut reader = Reader::from_reader(reader);
    let mut elem_buf = Vec::new();
    let mut text_buf = Vec::new();
    iter::from_fn(move || {
        let mut title = None;
        let mut ns = 0i16;
        loop {
            match reader.read_event(&mut elem_buf) {
                Ok(Event::Start(e)) => {
                    let name = e.name();
                    let mut read_text = || reader.read_text(name, &mut text_buf).unwrap();
                    if name == b"title" {
                        title = Some(read_text());
                    }
                    else if name == b"ns" {
                        ns = read_text().parse().unwrap();
                    }
                    else if name == b"text" {
                        if ns != 0 {
                            continue;
                        }
                        break Some((title.unwrap(), read_text()));
                    }
                },
                Ok(Event::Eof) => break None,
                Err(_) => panic!(),
                _ => (),
            }
            elem_buf.clear();
            text_buf.clear();
        }
    })
}


// Prepare static regular expressions
macro_rules! lazy_regex {
    { $( $name:ident : $re:expr ),* $(,)? } => {
        lazy_static! {
            $( static ref $name: Regex = Regex::new($re).unwrap(); )*
        }
    }
}

lazy_regex! {
    ENGLISH: r"(?ism)(?:^== *english *== *\n)(.*?)(?:^={1,2}[^=]|\z)",
    SECTION: r"(?m)^=+ *([^=]+?)( [0-9]+)? *=+ *\n",
    OBSOLETE: r"\b(?:obsolete|archaic|dated|rare)\b",
}


// Find the English block, if any
fn extract_english(text: &str) -> Option<&str> {
    ENGLISH.captures(text).map(|c| c.get(1).unwrap().as_str())
}


// Create a flat iterator over Markdown sections
fn sections(text: &str) -> impl Iterator<Item = (Option<String>, &str)> {
    let mut cur = 0;
    let mut title = None;
    let mut captures = SECTION.capture_locations();
    iter::from_fn(move || {
        if cur < text.len() {
            let (title, content) = match SECTION.captures_read_at(&mut captures, &text, cur) {
                Some(m) => {
                    let ptitle = title;
                    title = Some(captures.get(1).unwrap());
                    let content = &text[cur..m.start()];
                    cur = m.end();
                    (ptitle, content)
                },
                None => {
                    let content = &text[cur..];
                    cur = text.len();
                    (title, content)
                },
            };
            let title = title.map(|(i, j)| {
                let mut title = String::from(&text[i..j]);
                title.make_ascii_lowercase();  // Proper Noun == Proper noun
                title
            });
            Some((title, content))
        } else {
            None
        }
    })
}


// A simple structure to manage unique word identifiers
struct IdTable<'a> {
    vec: &'a mut Vec<String>,
    hash: BTreeMap<String, usize>,
}


impl<'a> IdTable<'a> {
    fn new(vec: &'a mut Vec<String>) -> Self {
        IdTable {
            vec,
            hash: Default::default(),
        }
    }

    fn get(&mut self, key: &str) -> usize {
        match self.hash.get(key) {
            Some(i) => *i,
            None => {
                let i = self.vec.len();
                self.vec.push(key.into());
                self.hash.insert(key.into(), i);
                i
            },
        }
    }
}


// Serialize a HashMap as a list of its values
fn serialize_values<K, V, S>(map: &HashMap<K, V>, serializer: S) -> Result<S::Ok, S::Error>
    where V: Serialize,
          S: Serializer,
{
    let mut seq = serializer.serialize_seq(Some(map.len()))?;
    for value in map.values() {
        seq.serialize_element(value)?;
    }
    seq.end()
}


// Structures to hold and serialize parsed Wiktionary data
#[derive(Default, Serialize)]
struct Relations {
    plural_of: HashSet<(usize, usize)>,  // directed edges
    #[serde(serialize_with = "serialize_values")]
    alt_forms: HashMap<usize, Vec<usize>>, // clusters
}


#[derive(Default, Serialize)]
struct Wiktionary {
    source: &'static str,
    license: &'static str,
    words: Vec<String>,
    pos: HashMap<String, HashSet<usize>>,
    rel: Relations,
}


impl Wiktionary {
    fn new() -> Self {
        Wiktionary {
            source: &"https://en.wiktionary.org",
            license: &"https://creativecommons.org/licenses/by-sa/3.0/",
            ..Default::default()
        }
    }
}


// A transforming view on wiktionary data
struct WiktionaryView<'a> {
    pos: &'a mut HashMap<String, HashSet<usize>>,
    id_table: IdTable<'a>,
    plural_of: &'a mut HashSet<(usize, usize)>,
    alt_forms: Partitioner<'a>,
}

impl<'a> WiktionaryView<'a> {
    fn new(wiktionary: &'a mut Wiktionary) -> Self {
        WiktionaryView {
            pos: &mut wiktionary.pos,
            id_table: IdTable::new(&mut wiktionary.words),
            plural_of: &mut wiktionary.rel.plural_of,
            alt_forms: Partitioner::new(&mut wiktionary.rel.alt_forms),
        }
    }

    fn word_id(&mut self, word: &str) -> usize {
        self.id_table.get(word)
    }
}


// Template handling
enum Error {
    MissingTemplateArgument,
}


struct TemplateContext<'a> {
    word: &'a str,
    section: Option<&'a str>,
    args: Vec<&'a str>,
    kwargs: HashMap<&'a str, &'a str>,
}


fn plural_of(view: &mut WiktionaryView, cxt: TemplateContext) -> Result<(), Error> {
    if let Some("noun") = cxt.section {
        if cxt.args.get(0).ok_or(Error::MissingTemplateArgument)? == &"en" {
            let id1 = view.word_id(cxt.word);
            let id2 = view.word_id(cxt.args.get(1).ok_or(Error::MissingTemplateArgument)?);
            view.plural_of.insert((id1, id2));
        }
    }
    Ok(())
}


fn alt_forms(view: &mut WiktionaryView, cxt: TemplateContext) -> Result<(), Error> {
    if cxt.args.get(0).ok_or(Error::MissingTemplateArgument)? == &"en" {
        let id1 = view.word_id(cxt.word);
        let id2 = view.word_id(cxt.args.get(1).ok_or(Error::MissingTemplateArgument)?);
        view.alt_forms.insert(id1, id2);
    }
    Ok(())
}


fn alter(view: &mut WiktionaryView, cxt: TemplateContext) -> Result<(), Error> {
    if cxt.args.get(0).ok_or(Error::MissingTemplateArgument)? == &"en" {
        let (forms, dialects) = match cxt.args.iter().position(|a| *a == "") {
            Some(delim) => {
                (&cxt.args[1..delim], &cxt.args[delim+1..])
            },
            None => {
                (&cxt.args[1..], &cxt.args[0..0])
            },
        };
        // Skip obsolete and rare alternative forms
        if dialects.iter().any(|arg| OBSOLETE.is_match(arg)) {
            return Ok(())
        }
        let id1 = view.word_id(cxt.word);
        for arg in forms {
            let id2 = view.word_id(arg);
            view.alt_forms.insert(id1, id2);
        }
    }
    Ok(())
}


// Template dispatching
static DISPATCHER: phf::Map<&'static str, fn(&mut WiktionaryView, TemplateContext) -> Result<(), Error>> = phf_map! {
    "plural of" => plural_of,
    "standard spelling of" => alt_forms,
    "alternative spelling of" => alt_forms,
    "standard form of" => alt_forms,
    "alternative form of" => alt_forms,
    "stand sp" => alt_forms,
    "alt sp" => alt_forms,
    "alt spelling" => alt_forms,
    "alt form" => alt_forms,
    "altform" => alt_forms,
    "alter" => alter,
};


// Explicitly list which parts of speech to collect
static POS_HEADERS: phf::Set<&'static str> = phf_set! {
    "noun",
    "verb",
    "adjective",
    "proper noun",
    "adverb",
    "interjection",
    "pronoun",
    "preposition",
    "conjuction",
    "determiner",
    "particle",
    "article",
};


// Collect specific wiktionary data
fn collect(reader: impl BufRead) -> Wiktionary {
    let mut wiktionary = Wiktionary::new();
    let mut view = WiktionaryView::new(&mut wiktionary);

    let reader = pages(reader);
    for (word, text) in reader {
        if word.ends_with("/translations") { continue; }
        if let Some(text) = extract_english(&text) {
            for (section, text) in sections(text) {
                // Templates
                template::process_templates(text, |name, args| {
                    if let Some(func) = DISPATCHER.get(name) {
                        let (args, kwargs) = template::decode_arguments(args);
                        func(&mut view, TemplateContext {
                            word: &word,
                            section: section.as_deref(),
                            args, kwargs
                        }).ok();
                    }
                });
                // Parts of speech
                if let Some(section) = section {
                    if POS_HEADERS.contains(&section) {
                        let id = view.word_id(&word);
                        let pos = view.pos.entry(section).or_default();
                        pos.insert(id);
                    }
                }
            }
        }
    }
    wiktionary
}


// stdin -> process -> stdout
// TODO: Error handling here and elsewhere
fn main() {
    let stdin = io::stdin();
    let wiktionary = collect(stdin.lock());
    serde_json::to_writer(io::stdout(), &wiktionary).unwrap();
}
