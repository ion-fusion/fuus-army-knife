// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use colorful::{Color, Colorful};
use diff::Result;

pub fn human_diff_lines<L: AsRef<str>, R: AsRef<str>>(left: L, right: R) -> String {
    let mut output = String::new();
    for diff in diff::lines(left.as_ref(), right.as_ref()) {
        match diff {
            Result::Left(value) => output.push_str(&format!("{}", format!("-{}\n", value).color(Color::Red))),
            Result::Both(value, _) => output.push_str(&format!(" {}\n", value)),
            Result::Right(value) => output.push_str(&format!("{}\n", format!("+{}", value).color(Color::Blue))),
        }
    }
    output
}
