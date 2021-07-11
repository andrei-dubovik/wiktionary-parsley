// Copyright (c) 2021, Andrey Dubovik <andrei@dubovik.eu>

// A rudimentary parser for Mediawiki templates

use std::collections::HashMap;


// Iterate over (possibly nested) mediawiki templates
// TODO: currently, nested templates and nowiki markup are left as is;
// in principle, these can be expanded
fn process_templates_inner<F>(text: &str, i: usize, func: &mut F) -> usize
    where F: FnMut(&str, &[(Option<&str>, &str)]) -> ()
{
    let mut i = i;
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut args = Vec::new();
    let mut start = i;
    let mut argname = None;
    while i < len {
        if (bytes[i] as i8) >= -0x40 {  // Unicode boundary
            // Collect argument name
            if bytes [i] == b'=' && argname.is_none() {
                argname = Some(&text[start..i]);
                i += 1;
                start = i;
            }
            // Collect argument
            else if bytes[i] == b'|' {
                args.push((argname, &text[start..i]));
                i += 1;
                start = i;
                argname = None;
            }
            // "{{{" should not occur
            else if i + 2 < len && &bytes[i..i+3] == b"{{{" {
                panic!("{}", "{{{ encountered")
            }
            // Enter new template
            else if i + 1 < len && &bytes[i..i+2] == b"{{" {
                i = process_templates_inner(text, i + 2, func);
            }
            // Process template and exit
            else if i + 1 < len && &bytes[i..i+2] == b"}}" {
                args.push((argname, &text[start..i]));
                //args[0] = args[0].trim();  // Trim name by default
                func(args[0].1.trim(), &args[1..]);
                return i + 2;
            }
            // Skip over <nowiki> segments
            else if i + 7 < len && &bytes[i..i+8] == b"<nowiki>" {
                match text[i+8..].find("</nowiki>") {
                    Some(j) => { i += j + 17; },
                    None => { i = len; },
                }
            }
            // Skip over <math> segments
            else if i + 5 < len && &bytes[i..i+6] == b"<math>" {
                match text[i+6..].find("</math>") {
                    Some(j) => { i += j + 13; },
                    None => { i = len; },
                }
            }
            else {
                i += 1;
            }
        }
        else {
            i += 1;
        }
    }
    i
}


// A public wrapper for a top-level call
pub fn process_templates<F>(text: &str, mut func: F)
    where F: FnMut(&str, &[(Option<&str>, &str)]) -> ()
{
    process_templates_inner(text, 0, &mut func);
}


// Assign a given element, resize if necessary
trait Setter<T> {
    fn set(&mut self, index: usize, value: T);
}


impl<T: Clone + Default> Setter<T> for Vec<T> {
    fn set(&mut self, index: usize, value: T) {
        if self.len() < index + 1 {
            self.resize(index + 1, Default::default());
        }
        self[index] = value;
    }
}


// Destructure mediawiki template parameters
pub fn decode_arguments<'a>(args: &'a[(Option<&str>, &str)]) -> (Vec<&'a str>, HashMap<&'a str, &'a str>) {
    let mut nargs = Vec::new();
    let mut kwargs = HashMap::new();
    let mut i = 1;
    for (name, value) in args {
        let value = value.trim();
        match name {
            None => {
                nargs.set(i - 1, value);
                i += 1;
            },
            Some(name) => {
                let name = name.trim();
                match usize::from_str_radix(name, 10) {
                    Ok(i) => {
                        nargs.set(i - 1, value);
                    },
                    Err(_) => {
                        kwargs.insert(name, value);
                    },
                }
            },
        }
    }
    (nargs, kwargs)
}
