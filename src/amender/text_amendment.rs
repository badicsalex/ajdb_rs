// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    semantic_info::TextAmendmentReplacement,
    structure::{Act, ChildrenCommon, LastChange, SAEBody, SubArticleElement},
    util::walker::SAEVisitorMut,
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, ModifyAct, NeedsFullReparse};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedTextAmendment {
    pub position: Reference,
    pub replacement: TextAmendmentReplacement,
}

impl ModifyAct for SimplifiedTextAmendment {
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
            .position
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
    amendment: &'a SimplifiedTextAmendment,
    change_entry: &'a LastChange,
    applied: bool,
}

impl<'a> SAEVisitorMut for Visitor<'a> {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.amendment.position.contains(position) {
            let from = &self.amendment.replacement.from;
            let to = &self.amendment.replacement.to;
            match &mut element.body {
                SAEBody::Text(text) => {
                    if text.contains(from) {
                        self.applied = true;
                        element.last_change = Some(self.change_entry.clone());
                        *text = normalized_replace(text, from, to)
                    }
                }
                SAEBody::Children { intro, wrap_up, .. } => {
                    if intro.contains(from) {
                        self.applied = true;
                        element.last_change = Some(self.change_entry.clone());
                        *intro = normalized_replace(intro, from, to);
                    }
                    if let Some(wrap_up) = wrap_up {
                        if wrap_up.contains(from) {
                            self.applied = true;
                            element.last_change = Some(self.change_entry.clone());
                            *wrap_up = normalized_replace(wrap_up, from, to);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

fn normalized_replace(text: &str, from: &str, to: &str) -> String {
    let result = text.replace(from, to);
    if to.is_empty() {
        // TODO: Maybe only remove spaces around the 'from' text?
        result.trim().replace("  ", " ")
    } else {
        result
    }
}

impl AffectedAct for SimplifiedTextAmendment {
    fn affected_act(&self) -> Result<ActIdentifier> {
        self.position
            .act()
            .ok_or_else(|| anyhow!("No act in reference in special phrase (TextAmendment)"))
    }
}

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use hun_law::util::singleton_yaml;

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
            cause: None,
        };

        let mod_1: SimplifiedTextAmendment = singleton_yaml::from_str(
            r#"
            position:
              act:
                year: 2012
                number: 1
              article: '1'
            replacement:
              from: "Article"
              to: "modified"
        "#,
        )
        .unwrap();
        mod_1.apply(&mut test_act, &change_entry).unwrap();
        assert!(mod_1.apply(&mut test_act, &change_entry).is_err());

        let mod_2: SimplifiedTextAmendment = singleton_yaml::from_str(
            r#"
            position:
              act:
                year: 2012
                number: 1
              article: '2'
            replacement:
              from: "Intro"
              to: "modified"
        "#,
        )
        .unwrap();
        mod_2.apply(&mut test_act, &change_entry).unwrap();
        assert!(mod_2.apply(&mut test_act, &change_entry).is_err());

        let mod_3: SimplifiedTextAmendment = singleton_yaml::from_str(
            r#"
            position:
              act:
                year: 2012
                number: 1
              article: '2'
            replacement:
              from: "wrap_up"
              to: "modified"
        "#,
        )
        .unwrap();
        mod_3.apply(&mut test_act, &change_entry).unwrap();
        assert!(mod_3.apply(&mut test_act, &change_entry).is_err());
    }
}
