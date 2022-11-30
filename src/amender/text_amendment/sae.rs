// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use anyhow::{anyhow, ensure, Result};
use hun_law::{
    identifier::IdentifierCommon,
    reference::Reference,
    semantic_info::TextAmendmentSAEPart,
    structure::{Act, ChildrenCommon, LastChange, SAEBody, SubArticleElement},
    util::walker::SAEVisitorMut,
};

use super::{text_replace::normalized_replace, NeedsFullReparse};

pub fn apply_sae_text_amendment(
    reference: &Reference,
    amended_part: &TextAmendmentSAEPart,
    from: &str,
    to: &str,
    act: &mut Act,
    change_entry: &LastChange,
) -> Result<NeedsFullReparse> {
    let mut visitor = Visitor {
        reference,
        amended_part,
        from,
        to,
        applied: false,
        change_entry,
    };
    act.walk_saes_mut(&mut visitor)?;
    ensure!(
        visitor.applied,
        "Text replacement @{reference:?} {amended_part:?} from={from:?} to={to:?} did not have an effect",
    );
    let article_ids = reference
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

struct Visitor<'a> {
    reference: &'a Reference,
    amended_part: &'a TextAmendmentSAEPart,
    from: &'a str,
    to: &'a str,
    change_entry: &'a LastChange,
    applied: bool,
}

impl<'a> SAEVisitorMut for Visitor<'a> {
    fn on_enter<IT: IdentifierCommon, CT: ChildrenCommon>(
        &mut self,
        position: &Reference,
        element: &mut SubArticleElement<IT, CT>,
    ) -> Result<()> {
        if self.reference.contains(position) {
            let from = &self.from;
            let to = &self.to;
            match &mut element.body {
                SAEBody::Text(text) => {
                    if self.amended_part == &TextAmendmentSAEPart::All {
                        if let Some(replaced) = normalized_replace(text, from, to) {
                            self.applied = true;
                            element.last_change = Some(self.change_entry.clone());
                            *text = replaced;
                        }
                    }
                }
                SAEBody::Children { intro, wrap_up, .. } => {
                    if self.amended_part == &TextAmendmentSAEPart::All
                        || self.amended_part == &TextAmendmentSAEPart::IntroOnly
                            && self.reference == position
                    {
                        if let Some(replaced) = normalized_replace(intro, from, to) {
                            self.applied = true;
                            element.last_change = Some(self.change_entry.clone());
                            *intro = replaced;
                        }
                    }
                    if let Some(wrap_up) = wrap_up {
                        if self.amended_part == &TextAmendmentSAEPart::All
                            || self.amended_part == &TextAmendmentSAEPart::WrapUpOnly
                                && self.reference == position
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

#[cfg(test)]
mod tests {
    use chrono::NaiveDate;
    use hun_law::{semantic_info::TextAmendment, structure::ChangeCause, util::singleton_yaml};

    use super::*;
    use crate::amender::ModifyAct;

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
              SAE:
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
              SAE:
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
              SAE:
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
}
