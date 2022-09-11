// This file is part of AJDB
// Copyright 2022, Alex Badics
// All rights reserved.

mod datatests;
pub mod test_utils;

#[allow(unused_macros)]
macro_rules! declare_test {
    (dir = $dir:expr, pattern = $pattern:expr) => {
        pub fn test_dir() -> String {
            std::path::Path::new(file!())
                .parent()
                .expect("No parent of calling module")
                .join($dir)
                .to_str()
                .expect("Path was not unicode somehow")
                .to_owned()
        }

        pub const FILE_PATTERN: &str = $pattern;
    };
}

pub(crate) use declare_test;

macro_rules! generate_harness{
    ($($id_first:ident$(::$id_rest:ident)*),* $(,)*) => {
        datatest_stable::harness!(
            $(
                datatests::$id_first$(::$id_rest)*::run_test,
                datatests::$id_first$(::$id_rest)*::test_dir(),
                datatests::$id_first$(::$id_rest)*::FILE_PATTERN,
            )*
        );
    }
}

generate_harness!(test_extract_modifications, test_apply_modifications,);
