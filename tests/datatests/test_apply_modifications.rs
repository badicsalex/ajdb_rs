// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::path::Path;

use ajdb::amender::{AppliableModification, AppliableModificationSet, AppliableModificationType};
use chrono::NaiveDate;
use hun_law::identifier::ActIdentifier;
use hun_law::structure::ActChild;
use hun_law::{structure::Act, util::singleton_yaml};
use serde::{Deserialize, Serialize};

use crate::declare_test;
use crate::test_utils::{ensure_eq, read_all};

declare_test!(dir = "data_apply_modifications", pattern = r"\.yml");

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestData {
    children_original: Vec<ActChild>,
    modifications: Vec<AppliableModificationType>,
    children_expected: Vec<ActChild>,
}

pub fn run_test(path: &Path) -> datatest_stable::Result<()> {
    let test_data: TestData = singleton_yaml::from_slice(&read_all(path)?)?;
    let mut act = Act {
        identifier: ActIdentifier {
            year: 2012,
            number: 1,
        },
        subject: "Az AJDB teszteléséről".to_string(),
        publication_date: NaiveDate::from_ymd(2012, 1, 1),
        preamble: "".to_owned(),
        contained_abbreviations: Default::default(),
        children: test_data.children_original,
    };
    act.add_semantic_info()?;
    let modifications = test_data
        .modifications
        .into_iter()
        .map(|modification| AppliableModification {
            source: None,
            modification,
        })
        .collect();
    AppliableModificationSet::apply_to_act(&mut act, modifications)?;
    ensure_eq(
        &test_data.children_expected,
        &act.children,
        "Wrong final act content",
    )?;
    Ok(())
}
