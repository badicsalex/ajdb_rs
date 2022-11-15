// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::str::CharIndices;

use anyhow::{anyhow, ensure, Result};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    semantic_info::{TextAmendment, TextAmendmentSAEPart},
    structure::{Act, ChildrenCommon, LastChange, SAEBody, SubArticleElement},
    util::walker::SAEVisitorMut,
};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};

impl ModifyAct for TextAmendment {
    fn apply(&self, act: &mut Act, change_entry: &LastChange) -> Result<NeedsFullReparse> {
        let mut visitor = Visitor {
            amendment: self,
            applied: false,
            change_entry,
        };
        act.walk_saes_mut(&mut visitor)?;
        ensure!(
            visitor.applied,
            "Text replacement {:?} did not have an effect",
            self
        );
        let article_ids = self
            .reference
            .article()
            .ok_or_else(|| anyhow!("No article in text amendment position"))?;
        if !article_ids.is_range() {
            let abbrevs_changed = act.add_semantic_info_to_article(article_ids.first_in_range())?;
            Ok(abbrevs_changed.into())
        } else {
            // TODO: Maybe not ask for a full reparse but handle this ourselves.
            //       Then again, this is just an optimization for very common cases.
            Ok(NeedsFullReparse::Yes)
        }
    }
}

struct Visitor<'a> {
    amendment: &'a TextAmendment,
    change_entry: &'a LastChange,
    applied: bool,
}

impl<'a> SAEVisitorMut for Visitor<'a> {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.amendment.reference.contains(position) {
            let from = &self.amendment.from;
            let to = &self.amendment.to;
            match &mut element.body {
                SAEBody::Text(text) => {
                    if self.amendment.amended_part == TextAmendmentSAEPart::All {
                        if let Some(replaced) = normalized_replace(text, from, to) {
                            self.applied = true;
                            element.last_change = Some(self.change_entry.clone());
                            *text = replaced;
                        }
                    }
                }
                SAEBody::Children { intro, wrap_up, .. } => {
                    if self.amendment.amended_part == TextAmendmentSAEPart::All
                        || self.amendment.amended_part == TextAmendmentSAEPart::IntroOnly
                            && self.amendment.reference == *position
                    {
                        if let Some(replaced) = normalized_replace(intro, from, to) {
                            self.applied = true;
                            element.last_change = Some(self.change_entry.clone());
                            *intro = replaced;
                        }
                    }
                    if let Some(wrap_up) = wrap_up {
                        if self.amendment.amended_part == TextAmendmentSAEPart::All
                            || self.amendment.amended_part == TextAmendmentSAEPart::WrapUpOnly
                                && self.amendment.reference == *position
                        {
                            if let Some(replaced) = normalized_replace(wrap_up, from, to) {
                                self.applied = true;
                                element.last_change = Some(self.change_entry.clone());
                                *wrap_up = replaced;
                            }
                        }
                    }
                }
            }
        }
        Ok(())
    }
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

fn normalized_replace(text: &str, from: &str, to: &str) -> Option<String> {
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

impl AffectedAct for TextAmendment {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.reference
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (TextAmendment)"))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use hun_law::{structure::ChangeCause, util::singleton_yaml};

    use super::*;

    const TEST_ACT: &str = r#"
        identifier:
          year: 2012
          number: 1
        subject: A tesztelésről
        preamble: A tesztelés nagyon fontos, és egyben kötelező
        publication_date: 2012-01-01
        children:
        - Article:
            identifier: 1
            children:
            - body: Article 1
        - Article:
            identifier: 2
            children:
            - identifier: '1'
              body: Paragraph
            - identifier: '2'
              body:
                intro: Intro
                children:
                  AlphabeticPoint:
                  - identifier: a
                    body: abcd
                  - identifier: b
                    body: efg
                wrap_up: wrap_up.
        "#;
    #[test]
    fn test_could_not_apply() {
        let mut test_act: Act = singleton_yaml::from_str(TEST_ACT).unwrap();
        let change_entry = LastChange {
            date: NaiveDate::from_ymd(2013, 2, 3),
            cause: ChangeCause::Other("Test".to_string()),
        };

        let mod_1: TextAmendment = singleton_yaml::from_str(
            r#"
            reference:
              act:
                year: 2012
                number: 1
              article: '1'
            from: "Article"
            to: "modified"
        "#,
        )
        .unwrap();
        mod_1.apply(&mut test_act, &change_entry).unwrap();
        assert!(mod_1.apply(&mut test_act, &change_entry).is_err());

        let mod_2: TextAmendment = singleton_yaml::from_str(
            r#"
            reference:
              act:
                year: 2012
                number: 1
              article: '2'
            from: "Intro"
            to: "modified"
        "#,
        )
        .unwrap();
        mod_2.apply(&mut test_act, &change_entry).unwrap();
        assert!(mod_2.apply(&mut test_act, &change_entry).is_err());

        let mod_3: TextAmendment = singleton_yaml::from_str(
            r#"
            reference:
              act:
                year: 2012
                number: 1
              article: '2'
            from: "wrap_up"
            to: "modified"
        "#,
        )
        .unwrap();
        mod_3.apply(&mut test_act, &change_entry).unwrap();
        assert!(mod_3.apply(&mut test_act, &change_entry).is_err());
    }

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
