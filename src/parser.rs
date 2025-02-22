// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::ast::*;
use crate::config::FusionConfig;
use crate::error::Error;
use crate::lexer::{FPair, FPairs, FusionLexer, Rule};
use crate::span::ShortSpan;
use crate::string_util::count_newlines;
use pest::Parser;
use std::path::Path;

pub type ParseResult = Result<Vec<Expr>, Error>;

pub fn parse<P: AsRef<Path>>(file_name: P, source: &str, config: &FusionConfig) -> ParseResult {
    // FusionParser::parse converts the string into a token stream using the grammar in grammar.pest.
    // The visit_pairs method then converts that token stream into the AST.
    let parse_result = FusionLexer::parse(Rule::file, source)
        .map_err(|error| err_generic!("{}{}", file_name.as_ref().display(), error));
    visit_pairs(parse_result?.next().unwrap().into_inner(), config)
}

macro_rules! atomic {
    ($expr_type:expr, $pair: ident) => {
        atomic!($expr_type, $pair, $pair.as_span())
    };
    ($expr_type:expr, $pair: expr, $span: expr) => {
        Ok(vec![Expr::Atomic(AtomicData::new(
            $expr_type,
            $span.into(),
            Vec::new(),
            $pair.as_str().into(),
        ))])
    };
}

macro_rules! result {
    ($expr_type:ident, $inner:expr) => {
        Ok(vec![Expr::$expr_type($inner)])
    };
}

fn visit_pairs(pairs: FPairs<'_>, config: &FusionConfig) -> ParseResult {
    let mut ast = Vec::new();
    for pair in pairs {
        ast.extend(visit_pair(pair, config)?);
    }
    Ok(ast)
}

fn visit_blob(pair: FPair<'_>) -> ParseResult {
    let span: ShortSpan = pair.as_span().into();
    let inner: String = pair.into_inner().next().unwrap().as_str().trim().into();
    atomic!(AtomicType::Blob, inner, span)
}

fn visit_clob(pair: FPair<'_>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let inner_exprs: Vec<ClobExpr> = visit_pairs(pair.into_inner(), config)?
        .into_iter()
        .map(|expr| match expr {
            Expr::MultilineString(data) => ClobExpr::MultilineString(data),
            Expr::Atomic(data) => match data.typ {
                AtomicType::QuotedString => ClobExpr::QuotedString(data),
                _ => unreachable!(),
            },
            Expr::Newlines(data) => ClobExpr::Newlines(data),
            _ => unreachable!(),
        })
        .collect();
    result!(Clob, ClobData::new(span.into(), Vec::new(), inner_exprs))
}

fn block_comment_lines(comment: &str) -> Vec<String> {
    // Chop off the '/*' and '*/'
    let comment = &comment[2..(comment.len() - 2)];
    comment
        .lines()
        .map(|line| {
            let mut line = line.trim();
            // Chop off starting asterisks
            if line.starts_with('*') {
                line = &line[1..];
                // Chop off a single space after the asterisk
                if line.starts_with(' ') {
                    line = &line[1..];
                }
            }
            line.to_string()
        })
        .collect()
}

fn visit_comment(pair: FPair<'_>) -> ParseResult {
    // Unfortunately, Pest strips out all the useful information for comments,
    // so re-parse the comment without the implicit comment rule
    let comment = pair.as_span().as_str();
    let span: ShortSpan = pair.as_span().into();

    let reparsed = FusionLexer::parse(Rule::any_comment, comment)?.next().unwrap();
    match reparsed.as_rule() {
        Rule::line_comment => Ok(vec![
            Expr::CommentLine(NonAnnotatedStringData::new(span, pair.as_str().trim_end().into())),
            Expr::Newlines(NewlinesData::new(span, 1)),
        ]),
        Rule::block_comment => result!(
            CommentBlock,
            NonAnnotatedStringListData::new(span, block_comment_lines(pair.as_str()))
        ),
        _ => unreachable!(),
    }
}

fn attach_annotations(exprs: Vec<Expr>, annotations: Vec<String>) -> Vec<Expr> {
    exprs
        .into_iter()
        .map(|expr| expr.attach_annotations(annotations.clone()))
        .collect()
}

fn visit_expr<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    let mut pairs: Vec<FPair<'i>> = pair.into_inner().collect();
    if pairs.len() == 2 {
        // [annotation, expression]
        let expr_pair = pairs.pop().unwrap();
        let annotation_pair = pairs.pop().unwrap();
        Ok(attach_annotations(
            visit_pair(expr_pair, config)?,
            annotation_pair
                .into_inner()
                .map(|ap| ap.as_span().as_str().to_string())
                .collect(),
        ))
    } else {
        // [expression]
        assert_eq!(1, pairs.len());
        visit_pair(pairs.pop().unwrap(), config)
    }
}

fn visit_list(pair: FPair<'_>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(List, ListData::new(span.into(), Vec::new(), sub_exprs))
}

fn visit_sexpr(pair: FPair<'_>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(SExpr, ListData::new(span.into(), Vec::new(), sub_exprs))
}

fn visit_short_string(pair: FPair<'_>) -> ParseResult {
    let span = pair.as_span();
    let string_val = pair.into_inner().as_str().to_string();
    atomic!(AtomicType::QuotedString, string_val, span)
}

fn visit_long_string(pair: FPair<'_>) -> ParseResult {
    let span = pair.as_span();
    let string_val = pair.into_inner().as_str().to_string();
    result!(
        MultilineString,
        MultilineStringData::new(span.into(), Vec::new(), string_val)
    )
}

fn visit_string_inner(pair: FPair<'_>) -> ParseResult {
    match pair.as_rule() {
        Rule::SHORT_STRING => visit_short_string(pair),
        Rule::LONG_STRING => visit_long_string(pair),
        _ => unreachable!("visit_string_inner with {:?}", pair),
    }
}

fn visit_string(pair: FPair<'_>) -> ParseResult {
    visit_string_inner(pair.into_inner().next().unwrap())
}

fn visit_structure_key(pair: FPair<'_>) -> ParseResult {
    let inner = pair.into_inner().next().unwrap();
    result!(
        StructKey,
        NonAnnotatedStringData::new(inner.as_span().into(), inner.as_str().into())
    )
}

fn visit_structure(pair: FPair<'_>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(Struct, ListData::new(span.into(), Vec::new(), sub_exprs))
}

fn visit_whitespace(pair: FPair<'_>) -> ParseResult {
    let span = pair.as_span();
    let newline_count = count_newlines(span.as_str());
    if newline_count > 0 {
        return result!(Newlines, NewlinesData::new(span.into(), newline_count as u16));
    }
    Ok(Vec::new())
}

fn visit_pair(pair: FPair<'_>, config: &FusionConfig) -> ParseResult {
    match pair.as_rule() {
        Rule::blob => visit_blob(pair),
        Rule::boolean => atomic!(AtomicType::Boolean, pair),
        Rule::clob => visit_clob(pair, config),
        Rule::COMMENT => visit_comment(pair),
        Rule::expr => visit_expr(pair, config),
        Rule::integer => atomic!(AtomicType::Integer, pair),
        Rule::list => visit_list(pair, config),
        Rule::null => atomic!(AtomicType::Null, pair),
        Rule::real => atomic!(AtomicType::Real, pair),
        Rule::sexpr => visit_sexpr(pair, config),
        Rule::string => visit_string(pair),
        Rule::structure => visit_structure(pair, config),
        Rule::struct_key => visit_structure_key(pair),
        Rule::struct_member => visit_pairs(pair.into_inner(), config),
        Rule::symbol => atomic!(AtomicType::Symbol, pair),
        Rule::timestamp => atomic!(AtomicType::Timestamp, pair),
        Rule::WHITESPACE => visit_whitespace(pair),
        Rule::EOI => Ok(Vec::new()),

        // Unreachable rules separated out so that if we add a new rule, we don't forget to edit this function
        Rule::annotation
        | Rule::annotations
        | Rule::any_comment
        | Rule::BINARY_INT
        | Rule::BLOB_INNER
        | Rule::BLOB_INNER_CHAR
        | Rule::block_comment
        | Rule::DECIMAL_INT
        | Rule::file
        | Rule::HEX_INT
        | Rule::line_comment
        | Rule::LONG_STRING
        | Rule::LONG_STRING_CHAR
        | Rule::LONG_STRING_INNER
        | Rule::SHORT_STRING
        | Rule::SHORT_STRING_CHAR
        | Rule::SHORT_STRING_INNER
        | Rule::struct_member_list
        | Rule::SYMBOL_FIRST_CHAR
        | Rule::SYMBOL_IDENT
        | Rule::SYMBOL_OPERATOR
        | Rule::SYMBOL_OPERATOR_CHARS
        | Rule::SYMBOL_QUOTE
        | Rule::SYMBOL_QUOTE_CHAR
        | Rule::SYMBOL_QUOTE_INNER
        | Rule::SYMBOL_TAIL_CHARS
        | Rule::TIMESTAMP
        | Rule::TS_DAY
        | Rule::TS_H
        | Rule::TS_HM
        | Rule::TS_HMS
        | Rule::TS_HMSM
        | Rule::TS_HOUR
        | Rule::TS_MILLI
        | Rule::TS_MINUTE
        | Rule::TS_MONTH
        | Rule::TS_OFFSET
        | Rule::TS_SECOND
        | Rule::TS_SEP
        | Rule::TS_SUFFIX
        | Rule::TS_UTC
        | Rule::TS_Y
        | Rule::TS_YEAR
        | Rule::TS_YM
        | Rule::TS_YMD
        | Rule::UNDERSCORE_SEP_BIN
        | Rule::UNDERSCORE_SEP_DIGITS
        | Rule::UNDERSCORE_SEP_HEX => unreachable!("{:#?}", pair.as_rule()),
    }
}

#[cfg(test)]
#[test]
fn test_count_newlines() {
    assert_eq!(0, count_newlines(""));
    assert_eq!(0, count_newlines("   "));
    assert_eq!(0, count_newlines(" \t  "));
    assert_eq!(0, count_newlines("foo"));
    assert_eq!(1, count_newlines("\n"));
    assert_eq!(2, count_newlines("\n\n"));
    assert_eq!(2, count_newlines("\r\r"));
    assert_eq!(1, count_newlines("\r\n"));
    assert_eq!(2, count_newlines("\r\n\r\n"));
    assert_eq!(3, count_newlines("\r\n\n\r\n"));
    assert_eq!(3, count_newlines("\r\n\r\r\n"));
}

#[cfg(test)]
#[test]
fn test_block_comment_lines() {
    assert_eq!(vec!["foo baz"], block_comment_lines("/* foo baz */"));
    assert_eq!(
        vec!["", "foo", "bar baz", ""],
        block_comment_lines(
            "/**
              * foo
              * bar baz
              */"
        )
    );
    assert_eq!(
        vec!["foo", "bar baz"],
        block_comment_lines(
            "/* foo
                bar baz */"
        )
    );
    assert_eq!(
        vec!["", "Foo", " some indented text", "bar baz", ""],
        block_comment_lines(
            "/**
              *Foo
              *  some indented text
              *bar baz
              */"
        )
    );
    assert_eq!(
        vec!["", "Foo", "  some indented text", "bar baz", ""],
        block_comment_lines(
            "/**
              * Foo
              *   some indented text
              * bar baz
              */"
        )
    );
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    use crate::config::new_default_config;
    use crate::diff_util::human_diff_lines;
    use crate::file::FusionFile;

    macro_rules! test {
        ($input:expr, $output:expr) => {
            let input = include_str!($input);
            let expected_output = include_str!($output).trim();
            let result = parse("test", input, &new_default_config());
            if let Err(error) = result {
                assert!(false, "Error: {}", error);
            } else {
                let file = FusionFile::new("test".into(), input.into(), result.unwrap());
                let actual_output = file.debug_ast();
                if expected_output != &actual_output {
                    let msg = format!(
                        "\nProcessing of {} didn't match expected output in {}:\n{}\n",
                        $input,
                        $output,
                        human_diff_lines(expected_output, actual_output)
                    );
                    assert!(false, "{}", msg);
                }
            }
        };
    }

    #[test]
    fn test_blob() {
        test!("../parser_tests/blob.input.fusion", "../parser_tests/blob.ast.txt");
    }

    #[test]
    fn test_boolean() {
        test!(
            "../parser_tests/boolean.input.fusion",
            "../parser_tests/boolean.ast.txt"
        );
    }

    #[test]
    fn test_comment() {
        test!(
            "../parser_tests/comment.input.fusion",
            "../parser_tests/comment.ast.txt"
        );
    }

    #[test]
    fn test_clob() {
        test!("../parser_tests/clob.input.fusion", "../parser_tests/clob.ast.txt");
    }

    #[test]
    fn test_mixed() {
        test!("../parser_tests/mixed.input.fusion", "../parser_tests/mixed.ast.txt");
    }

    #[test]
    fn test_integer() {
        test!(
            "../parser_tests/integer.input.fusion",
            "../parser_tests/integer.ast.txt"
        );
    }

    #[test]
    fn test_list() {
        test!("../parser_tests/list.input.fusion", "../parser_tests/list.ast.txt");
    }

    #[test]
    fn test_real() {
        test!("../parser_tests/real.input.fusion", "../parser_tests/real.ast.txt");
    }

    #[test]
    fn test_sexp() {
        test!("../parser_tests/sexp.input.fusion", "../parser_tests/sexp.ast.txt");
    }

    #[test]
    fn test_timestamp() {
        test!(
            "../parser_tests/timestamp.input.fusion",
            "../parser_tests/timestamp.ast.txt"
        );
    }

    #[test]
    fn test_operators() {
        test!(
            "../parser_tests/operators.input.fusion",
            "../parser_tests/operators.ast.txt"
        );
    }

    #[test]
    fn test_symbol() {
        test!("../parser_tests/symbol.input.fusion", "../parser_tests/symbol.ast.txt");
    }

    #[test]
    fn test_structure() {
        test!(
            "../parser_tests/structure.input.fusion",
            "../parser_tests/structure.ast.txt"
        );
        test!(
            "../parser_tests/complex-struct.input.fusion",
            "../parser_tests/complex-struct.ast.txt"
        );
    }
}
