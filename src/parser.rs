// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::*;
use crate::config::FusionConfig;
use crate::error::Error;
use crate::lexer::{FPair, FPairs, FusionLexer, Rule};
use crate::span::ShortSpan;
use crate::string_util::count_newlines;
use pest::Parser;
use std::path::Path;

pub type ParseResult = Result<Vec<Expr>, Error>;

pub fn parse<'i, P: AsRef<Path>>(
    file_name: P,
    source: &'i str,
    config: &FusionConfig,
) -> ParseResult {
    // FusionParser::parse converts the string into a token stream using the grammar in grammar.pest.
    // The visit_pairs method then converts that token stream into the AST.
    let parse_result = FusionLexer::parse(Rule::file, source)
        .map_err(|error| Error::Generic(format!("{}{}", file_name.as_ref().display(), error)));
    visit_pairs(parse_result?.next().unwrap().into_inner(), config)
}

macro_rules! simple_value_node {
    ($expr_type:ident, $pair: ident) => {
        Ok(vec![Expr::$expr_type(ValueNode::new(
            $pair.as_span().into(),
            $pair.as_str().into(),
        ))])
    };
}

macro_rules! result {
    ($expr_type:ident, $inner:expr) => {
        Ok(vec![Expr::$expr_type($inner)])
    };
}

fn visit_pairs<'i>(pairs: FPairs<'i>, config: &FusionConfig) -> ParseResult {
    let mut ast = Vec::new();
    for pair in pairs {
        ast.extend(visit_pair(pair, config)?);
    }
    Ok(ast)
}

fn visit_blob<'i>(pair: FPair<'i>) -> ParseResult {
    let span = pair.as_span();
    result!(
        Blob,
        ValueNode::new(
            span.into(),
            pair.into_inner().next().unwrap().as_str().trim().into()
        )
    )
}

fn visit_clob<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let inner_exprs: Result<Vec<Expr>, Error> = visit_pairs(pair.into_inner(), config);
    result!(Clob, ExpressionsNode::new(span.into(), inner_exprs?))
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

fn visit_comment<'i>(pair: FPair<'i>) -> ParseResult {
    // Unfortunately, Pest strips out all the useful information for comments,
    // so re-parse the comment without the implicit comment rule
    let comment = pair.as_span().as_str();
    let span: ShortSpan = pair.as_span().into();

    let reparsed = FusionLexer::parse(Rule::any_comment, comment)?
        .next()
        .unwrap();
    match reparsed.as_rule() {
        Rule::line_comment => Ok(vec![
            Expr::CommentLine(NonAnnotatedValue::new(
                span,
                pair.as_str().trim_end().into(),
            )),
            Expr::Newlines(NewlinesNode::new(span, 1)),
        ]),
        Rule::block_comment => result!(
            CommentBlock,
            NonAnnotatedValues::new(span, block_comment_lines(pair.as_str()),)
        ),
        _ => unreachable!(),
    }
}

fn attach_annotations<'i>(exprs: Vec<Expr>, annotations: &Vec<String>) -> Vec<Expr> {
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
            &annotation_pair
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

fn visit_list<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(List, ExpressionsNode::new(span.into(), sub_exprs))
}

fn visit_sexpr<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(SExpr, ExpressionsNode::new(span.into(), sub_exprs))
}

fn visit_short_string<'i>(pair: FPair<'i>) -> ParseResult {
    let span = pair.as_span();
    let string_val = pair.into_inner().as_str().to_string();
    result!(QuotedString, ValueNode::new(span.into(), string_val))
}

fn visit_long_string<'i>(pair: FPair<'i>) -> ParseResult {
    let span = pair.as_span();
    let string_val = pair.into_inner().as_str().to_string();
    result!(MultilineString, ValueNode::new(span.into(), string_val))
}

fn visit_string_inner<'i>(pair: FPair<'i>) -> ParseResult {
    match pair.as_rule() {
        Rule::SHORT_STRING => visit_short_string(pair),
        Rule::LONG_STRING => visit_long_string(pair),
        _ => unreachable!("visit_string_inner with {:?}", pair),
    }
}

fn visit_string<'i>(pair: FPair<'i>) -> ParseResult {
    visit_string_inner(pair.into_inner().next().unwrap())
}

fn visit_structure_key<'i>(pair: FPair<'i>) -> ParseResult {
    let inner = pair.into_inner().next().unwrap();
    simple_value_node!(StructKey, inner)
}

fn visit_structure_member<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(StructMember, StructMemberNode::new(span.into(), sub_exprs))
}

fn visit_structure<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    let span = pair.as_span();
    let sub_exprs: Vec<Expr> = visit_pairs(pair.into_inner(), config)?;
    result!(Struct, ExpressionsNode::new(span.into(), sub_exprs))
}

fn visit_whitespace<'i>(pair: FPair<'i>) -> ParseResult {
    let span = pair.as_span();
    let newline_count = count_newlines(span.as_str());
    if newline_count > 0 {
        return result!(
            Newlines,
            NewlinesNode::new(span.into(), newline_count as u16)
        );
    }
    Ok(Vec::new())
}

fn visit_pair<'i>(pair: FPair<'i>, config: &FusionConfig) -> ParseResult {
    match pair.as_rule() {
        Rule::blob => visit_blob(pair),
        Rule::boolean => simple_value_node!(Boolean, pair),
        Rule::clob => visit_clob(pair, config),
        Rule::COMMENT => visit_comment(pair),
        Rule::expr => visit_expr(pair, config),
        Rule::integer => simple_value_node!(Integer, pair),
        Rule::list => visit_list(pair, config),
        Rule::null => simple_value_node!(Null, pair),
        Rule::real => simple_value_node!(Real, pair),
        Rule::sexpr => visit_sexpr(pair, config),
        Rule::string => visit_string(pair),
        Rule::structure => visit_structure(pair, config),
        Rule::struct_key => visit_structure_key(pair),
        Rule::struct_member => visit_structure_member(pair, config),
        Rule::symbol => simple_value_node!(Symbol, pair),
        Rule::timestamp => simple_value_node!(Timestamp, pair),
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
    use crate::file::FusionFile;
    use crate::test_util::human_diff_lines;

    macro_rules! test {
        ($input:expr, $output:expr) => {
            let input = include_str!($input);
            let expected_output = include_str!($output).trim();
            let result = parse("test", input, &new_default_config());
            if let Err(error) = result {
                assert!(false, "Error: {}", error);
            } else {
                let file = FusionFile::test_file_with_ast(input, result.unwrap());
                let actual_output = file.debug_ast();
                if expected_output != &actual_output {
                    let msg = format!(
                        "\nProcessing of {} didn't match expected output in {}:\n{}\n",
                        $input,
                        $output,
                        human_diff_lines(expected_output, actual_output)
                    );
                    assert!(false, msg);
                }
            }
        };
    }

    #[test]
    fn test_blob() {
        test!(
            "../ast_tests/blob.input.fusion",
            "../ast_tests/blob.ast.txt"
        );
    }

    #[test]
    fn test_boolean() {
        test!(
            "../ast_tests/boolean.input.fusion",
            "../ast_tests/boolean.ast.txt"
        );
    }

    #[test]
    fn test_comment() {
        test!(
            "../ast_tests/comment.input.fusion",
            "../ast_tests/comment.ast.txt"
        );
    }

    #[test]
    fn test_clob() {
        test!(
            "../ast_tests/clob.input.fusion",
            "../ast_tests/clob.ast.txt"
        );
    }

    #[test]
    fn test_mixed() {
        test!(
            "../ast_tests/mixed.input.fusion",
            "../ast_tests/mixed.ast.txt"
        );
    }

    #[test]
    fn test_integer() {
        test!(
            "../ast_tests/integer.input.fusion",
            "../ast_tests/integer.ast.txt"
        );
    }

    #[test]
    fn test_list() {
        test!(
            "../ast_tests/list.input.fusion",
            "../ast_tests/list.ast.txt"
        );
    }

    #[test]
    fn test_real() {
        test!(
            "../ast_tests/real.input.fusion",
            "../ast_tests/real.ast.txt"
        );
    }

    #[test]
    fn test_sexp() {
        test!(
            "../ast_tests/sexp.input.fusion",
            "../ast_tests/sexp.ast.txt"
        );
    }

    #[test]
    fn test_timestamp() {
        test!(
            "../ast_tests/timestamp.input.fusion",
            "../ast_tests/timestamp.ast.txt"
        );
    }

    #[test]
    fn test_operators() {
        test!(
            "../ast_tests/operators.input.fusion",
            "../ast_tests/operators.ast.txt"
        );
    }

    #[test]
    fn test_symbol() {
        test!(
            "../ast_tests/symbol.input.fusion",
            "../ast_tests/symbol.ast.txt"
        );
    }

    #[test]
    fn test_structure() {
        test!(
            "../ast_tests/structure.input.fusion",
            "../ast_tests/structure.ast.txt"
        );
        test!(
            "../ast_tests/complex-struct.input.fusion",
            "../ast_tests/complex-struct.ast.txt"
        );
    }
}
