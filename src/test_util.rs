// Copyright Ion Fusion contributors. All Rights Reserved.
use colorful::{Color, Colorful};
use diff;

pub fn human_diff_lines<L: AsRef<str>, R: AsRef<str>>(left: L, right: R) -> String {
    let mut output = String::new();
    for diff in diff::lines(left.as_ref(), right.as_ref()) {
        match diff {
            diff::Result::Left(value) => {
                output.push_str(&format!("{}", format!("-{}\n", value).color(Color::Red)))
            }
            diff::Result::Both(value, _) => output.push_str(&format!(" {}\n", value)),
            diff::Result::Right(value) => {
                output.push_str(&format!("{}\n", format!("+{}", value).color(Color::Blue)))
            }
        }
    }
    output
}
