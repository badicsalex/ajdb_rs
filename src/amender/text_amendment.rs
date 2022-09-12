// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use hun_law::{
    identifier::{ActIdentifier, IdentifierCommon},
    reference::Reference,
    semantic_info::TextAmendmentReplacement,
    structure::{Act, SAEBody, SubArticleElement},
    util::walker::SAEVisitorMut,
};
use serde::{Deserialize, Serialize};

use super::{AffectedAct, Modify};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SimplifiedTextAmendment {
    pub position: Reference,
    pub replacement: TextAmendmentReplacement,
}

impl Modify<Act> for SimplifiedTextAmendment {
    fn apply(&self, act: &mut Act) -> Result<()> {
        let mut visitor = Visitor {
            amendment: self,
            applied: false,
        };
        act.walk_saes_mut(&mut visitor)?;
        ensure!(
            visitor.applied,
            "Text replacement {:?} did not have an effect",
            self
        );
        Ok(())
    }
}

struct Visitor<'a> {
    amendment: &'a SimplifiedTextAmendment,
    applied: bool,
}

impl<'a> SAEVisitorMut for Visitor<'a> {
    fn on_enter<IT: IdentifierCommon, CT>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.amendment.position.contains(position) {
            let from = &self.amendment.replacement.from;
            let to = &self.amendment.replacement.to;
            match &mut element.body {
                SAEBody::Text(text) => {
                    self.applied = self.applied || text.contains(from);
                    *text = text.replace(from, to)
                }
                SAEBody::Children { intro, wrap_up, .. } => {
                    self.applied = self.applied || intro.contains(from);
                    *intro = intro.replace(from, to);
                    if let Some(wrap_up) = wrap_up {
                        self.applied = self.applied || wrap_up.contains(from);
                        *wrap_up = wrap_up.replace(from, to);
                    }
                }
            }
        }
        Ok(())
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
        mod_1.apply(&mut test_act).unwrap();
        assert!(mod_1.apply(&mut test_act).is_err());

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
        mod_2.apply(&mut test_act).unwrap();
        assert!(mod_2.apply(&mut test_act).is_err());

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
        mod_3.apply(&mut test_act).unwrap();
        assert!(mod_3.apply(&mut test_act).is_err());
    }
}
