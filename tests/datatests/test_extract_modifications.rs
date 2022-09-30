// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

use std::collections::BTreeMap;
use std::path::Path;

use ajdb::amender::{AppliableModification, AppliableModificationSet};
use hun_law::{structure::Act, util::singleton_yaml};

use crate::declare_test;
use crate::test_utils::{clean_quoted_blocks, ensure_eq, parse_txt_as_act, read_all};

declare_test!(dir = "data_extract_modifications", pattern = r"\.txt");

pub type TestData = BTreeMap<String, BTreeMap<String, Vec<AppliableModification>>>;

pub fn run_test(path: &Path) -> datatest_stable::Result<()> {
    let mut act: Act = parse_txt_as_act(path)?;
    act.add_semantic_info()?;
    act.convert_block_amendments()?;
    // Clear remaining quoted blocks to make failing output a bit smaller
    clean_quoted_blocks(&mut act);
    let mut result: TestData = Default::default();
    for date in act.publication_date.iter_days().take(365) {
        let mut modification_set = AppliableModificationSet::default();
        modification_set.add(&act, date)?;

        let modifications = modification_set.get_modifications();
        if !modifications.is_empty() {
            let transformed_modifications = modifications
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();
            result.insert(date.to_string(), transformed_modifications);
        }
    }
    let expected: TestData = singleton_yaml::from_slice(&read_all(path.with_extension("yml"))?)?;
    ensure_eq(&expected, &result, "Wrong extracted modifiations")?;
    Ok(())
}
