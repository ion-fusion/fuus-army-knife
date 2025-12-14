// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

mod fixup;
mod formatter;

use crate::ast::Expr;
use crate::config::FusionConfig;
use crate::format::formatter::Formatter;

/// Formats the given AST into a String using the provided `FusionConfig`
pub fn format(fusion_config: &FusionConfig, ast: &Vec<Expr>) -> String {
    let mut formatter = Formatter::new(fusion_config);
    if fusion_config.newline_fix_up_mode() {
        formatter.format(&fixup::fixup_ast(ast));
    } else {
        formatter.format(ast);
    }
    formatter.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::new_default_config;
    use crate::diff_util::human_diff_lines;
    use crate::file::FusionFileContent;

    macro_rules! test {
        ($input:expr, $output:expr) => {
            let default_config = new_default_config();
            test!(&default_config, $input, $output);
        };
        ($config:expr, $input:expr, $output:expr) => {
            let input = include_str!($input);
            let expected_output = include_str!($output).trim();
            let file = FusionFileContent::new("test".into(), input.into())
                .parse($config)
                .unwrap_or_else(|error| panic!("Error: {}", error));
            let actual_output = format($config, &file.ast).trim().to_string();
            if expected_output != &actual_output {
                let msg = format!(
                    "\nProcessing of {} didn't match expected output in {}:\n{}\n",
                    $input,
                    $output,
                    human_diff_lines(expected_output, actual_output)
                );
                assert!(false, "{}", msg);
            }
        };
    }

    #[test]
    fn clob() {
        test!(
            "../../format_tests/clob.input.fusion",
            "../../format_tests/clob.formatted.fusion"
        );
    }

    #[test]
    fn comment_block() {
        test!(
            "../../format_tests/comment_block.input.fusion",
            "../../format_tests/comment_block.formatted.fusion"
        );
    }

    #[test]
    fn multiline_string() {
        let mut config = new_default_config();
        config.format_multiline_string_contents = true;
        test!(
            &config,
            "../../format_tests/multiline_string.input.fusion",
            "../../format_tests/multiline_string.formatted.fusion"
        );
    }

    #[test]
    fn multiline_string_no_change_whitespace() {
        test!(
            "../../format_tests/multiline_string_no_change_whitespace.input.fusion",
            "../../format_tests/multiline_string_no_change_whitespace.formatted.fusion"
        );
    }

    #[test]
    fn simple_continuations() {
        test!(
            "../../format_tests/simple_continuations.input.fusion",
            "../../format_tests/simple_continuations.formatted.fusion"
        );
    }

    #[test]
    fn complex_continuations() {
        test!(
            "../../format_tests/complex_continuations.input.fusion",
            "../../format_tests/complex_continuations.formatted.fusion"
        );
    }

    #[test]
    fn simple_function() {
        test!(
            "../../format_tests/simple_function.input.fusion",
            "../../format_tests/simple_function.formatted.fusion"
        );
    }

    #[test]
    fn structs() {
        test!(
            "../../format_tests/structs.input.fusion",
            "../../format_tests/structs.formatted.fusion"
        );
    }

    #[test]
    fn lists() {
        test!(
            "../../format_tests/lists.input.fusion",
            "../../format_tests/lists.formatted.fusion"
        );
    }

    #[test]
    fn fix_up() {
        test!(
            "../../format_tests/fix_up.input.fusion",
            "../../format_tests/fix_up.formatted.fusion"
        );
    }

    #[test]
    fn misc() {
        test!(
            "../../format_tests/misc.input.fusion",
            "../../format_tests/misc.formatted.fusion"
        );
    }
}
