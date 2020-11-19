// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::*;
use crate::error::Error;
use crate::lexer::{FusionLexer, Rule};
use pest::iterators::{Pair, Pairs};
use pest::Parser;
use std::path::Path;

pub fn parse<'i, P: AsRef<Path>>(file_name: P, source: &'i str) -> Result<Vec<Expr<'i>>, Error> {
    // FusionParser::parse converts the string into a token stream using the grammar in grammar.pest.
    // The visit_pairs method then converts that token stream into the AST.
    let parse_result = FusionLexer::parse(Rule::file, source)
        .map_err(|error| Error::Generic(format!("{}{}", file_name.as_ref().display(), error)));
    visit_pairs(parse_result?.next().unwrap().into_inner())
}

fn visit_pairs<'i>(pairs: Pairs<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    println!("{:#?}", pairs); // TODO remove
    let mut ast = Vec::new();
    for pair in pairs {
        ast.extend(visit_pair(pair)?);
    }
    Ok(ast)
}

fn visit_clob<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

fn visit_comment<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

fn visit_expr<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

fn visit_list<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

fn visit_sexpr<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

fn visit_string<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

fn visit_structure<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    unimplemented!()
}

macro_rules! simple_value_node {
    ($expr_type:ident, $pair: ident) => {
        Ok(vec![Expr::$expr_type(ValueNode::new(
            $pair.as_span(),
            Vec::new(),
            $pair.as_str().into(),
        ))])
    };
}

fn visit_pair<'i>(pair: Pair<'i, Rule>) -> Result<Vec<Expr<'i>>, Error> {
    let span = pair.as_span();
    match pair.as_rule() {
        Rule::blob => simple_value_node!(Blob, pair),
        Rule::boolean => simple_value_node!(Boolean, pair),
        Rule::clob => visit_clob(pair),
        Rule::COMMENT => visit_comment(pair),
        Rule::expr => visit_expr(pair),
        Rule::integer => simple_value_node!(Integer, pair),
        Rule::list => visit_list(pair),
        Rule::null => simple_value_node!(Null, pair),
        Rule::real => simple_value_node!(Real, pair),
        Rule::sexpr => visit_sexpr(pair),
        Rule::string => visit_string(pair),
        Rule::structure => visit_structure(pair),
        Rule::symbol => simple_value_node!(Symbol, pair),
        Rule::timestamp => simple_value_node!(Timestamp, pair),
        Rule::WHITESPACE => Ok(vec![Expr::Whitespace(NonAnnotatedValue::new(
            span,
            pair.as_str().into(),
        ))]),

        // Unreachable rules separated out so that if we add a new rule, we don't forget to edit this function
        Rule::annotation
        | Rule::annotations
        | Rule::BINARY_INT
        | Rule::block_comment
        | Rule::DECIMAL_INT
        | Rule::file
        | Rule::EOI
        | Rule::HEX_INT
        | Rule::line_comment
        | Rule::LONG_STRING
        | Rule::SHORT_STRING
        | Rule::SHORT_STRING_CHAR
        | Rule::SHORT_STRING_INNER
        | Rule::struct_member
        | Rule::struct_member_list
        | Rule::SYMBOL_FIRST_CHAR
        | Rule::SYMBOL_IDENT
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
