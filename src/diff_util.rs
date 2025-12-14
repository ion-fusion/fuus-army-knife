// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use colorful::{Color, Colorful};
use diff::Result;
use std::fmt::Write;

pub fn human_diff_lines<L: AsRef<str>, R: AsRef<str>>(left: L, right: R) -> String {
    let mut output = String::new();
    for diff in diff::lines(left.as_ref(), right.as_ref()) {
        match diff {
            Result::Left(value) => writeln!(output, "{}", format!("-{value}").color(Color::Red)),
            Result::Both(value, _) => writeln!(output, " {value}"),
            Result::Right(value) => writeln!(output, "{}", format!("+{value}").color(Color::Blue)),
        }
        .expect("output is a string");
    }
    output
}
