// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::str::CharIndices;

pub fn normalized_replace(text: &str, from: &str, to: &str) -> Option<String> {
    let from = from.trim();
    let to = to.trim();
    let mut result = None;
    let mut last_matched_index = 0;
    for matched_index in WholeWordFinderIterator::new(text, from) {
        let result = result.get_or_insert_with(String::new);
        result.push_str(&text[last_matched_index..matched_index]);
        result.push_str(to);
        last_matched_index = matched_index + from.len();
    }
    if let Some(result) = &mut result {
        result.push_str(&text[last_matched_index..]);
        // XXX: This is a workaround and should probably be handled i nthe 'for' above
        *result = result.trim().replace("  ", " ");
    }
    result
}

struct WordBoundaryIterator<'a> {
    chars_iter: CharIndices<'a>,
    last_was_alphanumeric: bool,
}

impl<'a> WordBoundaryIterator<'a> {
    pub fn new(s: &'a str) -> Self {
        Self {
            chars_iter: s.char_indices(),
            last_was_alphanumeric: false,
        }
    }
}

impl<'a> Iterator for WordBoundaryIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        for (i, c) in self.chars_iter.by_ref() {
            if is_hun_alphanumeric(c) {
                if !self.last_was_alphanumeric {
                    self.last_was_alphanumeric = true;
                    return Some(i);
                } else {
                    /* continue to next character */
                }
            } else {
                self.last_was_alphanumeric = false;
                return Some(i);
            }
        }
        None
    }
}

struct WholeWordFinderIterator<'a> {
    s: &'a str,
    needle: &'a str,
    words_iter: WordBoundaryIterator<'a>,
}

impl<'a> WholeWordFinderIterator<'a> {
    pub fn new(s: &'a str, needle: &'a str) -> Self {
        Self {
            s,
            needle,
            words_iter: WordBoundaryIterator::new(s),
        }
    }
}

impl<'a> Iterator for WholeWordFinderIterator<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        self.words_iter.by_ref().find(|&pos| {
            self.s[pos..].starts_with(self.needle)
                && !index_is_alphanumeric(self.s, pos + self.needle.len())
        })
    }
}

fn index_is_alphanumeric(s: &str, pos: usize) -> bool {
    match s[pos..].chars().next() {
        Some(c) => is_hun_alphanumeric(c),
        None => false,
    }
}
fn is_hun_alphanumeric(c: char) -> bool {
    c.is_ascii_alphanumeric()
        || [
            'á', 'é', 'ő', 'ú', 'ó', 'ü', 'ö', 'ű', 'í', 'Á', 'É', 'Ő', 'Ú', 'Ó', 'Ü', 'Ö', 'Ű',
            'Í',
        ]
        .contains(&c)
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_normalized_replace() {
        assert_eq!(
            normalized_replace("Egy kettő három", "kettő", "öt").unwrap(),
            "Egy öt három"
        );
        assert_eq!(
            normalized_replace("Egy kettő", "kettő", "öt").unwrap(),
            "Egy öt"
        );
        assert_eq!(
            normalized_replace("kettő három", "kettő", "öt").unwrap(),
            "öt három"
        );
        assert!(normalized_replace("Az hatásos lenne", "hat", "hét").is_none());
        assert!(normalized_replace("Az meghat majd", "hat", "hét").is_none());
        assert_eq!(
            normalized_replace("Egy, kettő, három", "kettő", "öt").unwrap(),
            "Egy, öt, három"
        );
        assert_eq!(
            normalized_replace("aaa aaa aaa", "aaa", "bbbb").unwrap(),
            "bbbb bbbb bbbb"
        );
        assert_eq!(
            normalized_replace("aaa aaa aaa", "aaa", "aaa aaa").unwrap(),
            "aaa aaa aaa aaa aaa aaa"
        );
    }
}
