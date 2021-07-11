# wiktionary-parsley

Yet another parser for English wiktionary.

# Usage

E.g.,

```
cat enwiktionary-20210601-pages-articles.xml | ./wiktionary-parsley > enwiktionary.json
```

On a Ryzon 3600 this runs in under a minute, sans the time to read the file from disk.

# JSON Structure

A compact JSON structure is used. Words are listed only once, in `words`, and the rest of the data uses integer zero-based indices that reference `words`. Additionally, dictionaries are used in the top-level hierarchy only. The structure is as follows:

- `source`: "https://en.wiktionary.org";
- `license`: "https://creativecommons.org/licenses/by-sa/3.0/"
- `words`: a list of all parsed words;
- `pos`: parts of speach, only the listed ones are extracted;
  - `noun`: a list of nouns, similar for the rest;
  - `verb`;
  - `adjective`;
  - `proper noun`;
  - `adverb`;
  - `interjection`;
  - `pronoun`;
  - `preposition`;
  - `conjuction`;
  - `determiner`;
  - `particle`;
  - `article`;
- `rel`: relationships between words;
  - `plural_of`: a list of directed edges `(i, j)`, where `i` is a plural of `j`;
  - `alt_forms`: a list of clusters of words that are alternative forms of one another. (Obsolete and rare alternative forms are skipped.)

A processed 2021-06-01 dump can be found here: https://dubovik.eu/static/enwiktionary-20210601.json.zst.
