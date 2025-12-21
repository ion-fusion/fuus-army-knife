// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::ast::{
    AtomicData, AtomicType, ClobData, ClobExpr, CountItemsBeforeNewline, CountNewlines, Expr, ListData,
    MultilineStringData, NewlinesData, NonAnnotatedStringData, NonAnnotatedStringListData,
};
use crate::config::FusionConfig;
use crate::string_util::{
    already_has_whitespace_before_cursor, find_cursor_pos, format_indented_multiline, last_is_one_of, repeat,
    trim_indent,
};
use std::fmt::Write;

pub struct Formatter<'i> {
    config: &'i FusionConfig,
    output: String,
}
impl<'i> Formatter<'i> {
    pub fn new(config: &'i FusionConfig) -> Formatter<'i> {
        Formatter {
            config,
            output: String::new(),
        }
    }

    pub fn format(&mut self, exprs: &Vec<Expr>) {
        self.visit_exprs(exprs, 0);
    }

    pub fn finish(self) -> String {
        self.output
            .lines()
            .map(str::trim_end)
            .fold(String::new(), |l, r| l + r + "\n")
    }

    fn visit_exprs(&mut self, exprs: &Vec<Expr>, next_indent: usize) {
        for expr in exprs {
            self.visit_expr(expr, next_indent);
        }
    }

    fn visit_expr(&mut self, expr: &Expr, next_indent: usize) {
        match expr {
            Expr::Atomic(data) => self.visit_atomic(data),
            Expr::Clob(data) => self.visit_clob(data, next_indent),
            Expr::CommentBlock(data) => self.visit_comment_block(data, next_indent),
            Expr::CommentLine(data) => self.visit_comment_line(data, next_indent),
            Expr::List(data) => self.visit_list(data),
            Expr::MultilineString(data) => self.visit_multiline_string(data),
            Expr::Newlines(data) => self.visit_newlines(data, next_indent),
            Expr::SExpr(data) => self.visit_sexpr(data),
            Expr::Struct(data) => self.visit_struct(data),
            Expr::StructKey(data) => self.visit_struct_key(data),
        }
    }

    fn visit_annotations(&mut self, annotations: &[String]) {
        for annotation in annotations {
            self.output.push_str(annotation);
        }
    }

    fn visit_atomic(&mut self, data: &AtomicData) {
        self.visit_annotations(&data.annotations);
        match data.typ {
            AtomicType::QuotedString => write!(self.output, "\"{}\"", data.value).expect("output is a string"),
            _ => self.output.push_str(&data.value),
        }
    }

    fn visit_clob(&mut self, data: &ClobData, next_indent: usize) {
        self.visit_annotations(&data.annotations);
        self.output.push_str("{{");

        let continuation_indent = match (&data.clobs[..]).count_items_before_newline() {
            0 => next_indent + 1,
            _ => find_cursor_pos(&self.output) + 1,
        };
        for expr in &data.clobs {
            if !expr.is_newlines() && !already_has_whitespace_before_cursor(&self.output) {
                self.output.push(' ');
            }
            match *expr {
                ClobExpr::Newlines(ref data) => self.visit_newlines(data, continuation_indent),
                ClobExpr::MultilineString(ref data) => self.visit_clob_string(data),
                ClobExpr::QuotedString(ref data) => self.visit_atomic(data),
            }
        }
        if !already_has_whitespace_before_cursor(&self.output) {
            self.output.push(' ');
        }
        self.output.push_str("}}");
    }

    fn visit_clob_string(&mut self, data: &MultilineStringData) {
        self.output.push_str("'''");
        self.output.push_str(&data.value);
        self.output.push_str("'''");
    }

    fn visit_comment_block(&mut self, data: &NonAnnotatedStringListData, _next_indent: usize) {
        let continuation_indent = find_cursor_pos(&self.output) + 1;
        self.output.push_str("/*");
        if data.value.len() == 1 {
            self.output.push(' ');
            self.output.push_str(data.value[0].trim());
            self.output.push(' ');
        } else {
            for i in 0..data.value.len() {
                let line = &data.value[i];
                if i > 0 && line.trim().is_empty() && i == data.value.len() - 1 {
                    break;
                } else if i > 0 {
                    self.output.push_str(&repeat(' ', continuation_indent));
                    self.output.push('*');
                }
                if !line.trim().is_empty() {
                    self.output.push(' ');
                }
                self.output.push_str(line);
                self.output.push('\n');
            }
            if last_is_one_of(&self.output, &['\n']) {
                self.output.push_str(&repeat(' ', continuation_indent));
            }
        }
        self.output.push_str("*/");
    }

    fn visit_comment_line(&mut self, data: &NonAnnotatedStringData, next_indent: usize) {
        self.output.push_str(&data.value);
        self.output.push_str(&newline(0, next_indent));
    }

    fn visit_multiline_string(&mut self, data: &MultilineStringData) {
        self.visit_annotations(&data.annotations);
        let continuation_indent = find_cursor_pos(&self.output);
        self.output.push_str("'''");
        let value = if self.config.format_multiline_string_contents {
            format_indented_multiline(&trim_indent(&data.value), continuation_indent)
        } else {
            data.value.clone()
        };
        self.output.push_str(value.trim_end_matches([' ', '\t']));
        if last_is_one_of(&self.output, &['\n']) {
            self.output.push_str(&repeat(' ', continuation_indent));
        }
        self.output.push_str("'''");
    }

    fn visit_newlines(&mut self, data: &NewlinesData, next_indent: usize) {
        self.output.push_str(&newline(data.newline_count as usize, next_indent));
    }

    // Complicated logic for determining whitespace between s-expression members due to
    // the inconsistent formatting of `|` lambda argument lists
    fn bind_whitespace(exprs: &[Expr]) -> Vec<(&Expr, bool)> {
        let is_arg_symbol = |expr: &Expr| expr.symbol_value().is_some_and(|v| v == "|");

        let mut bound = Vec::new();
        let mut first_is_arg_list = false;
        for i in 0..exprs.len() {
            let expr = &exprs[i];
            let is_first = i == 0;
            let is_last = i != exprs.len() - 1;

            if is_first && is_arg_symbol(expr) {
                bound.push((expr, false));
                first_is_arg_list = true;
            } else if !is_first && first_is_arg_list && is_arg_symbol(expr) {
                bound.push((expr, true));
            } else {
                let next_ends_arg_list = match exprs.get(i + 1) {
                    Some(next) => first_is_arg_list && is_arg_symbol(next),
                    None => false,
                };
                bound.push((expr, !next_ends_arg_list && is_last && !expr.is_newlines()));
            }
        }
        bound
    }

    fn visit_sexpr(&mut self, data: &ListData) {
        self.visit_annotations(&data.annotations);
        let opening_indent = find_cursor_pos(&self.output);
        self.output.push('(');

        let bound = Formatter::bind_whitespace(&data.items);
        if !bound.is_empty() {
            let continuation_indent = calculate_continuation_indent(self.config, &data.items, opening_indent);
            for (item, add_space) in bound {
                self.visit_expr(item, continuation_indent);
                if add_space {
                    self.output.push(' ');
                }
            }
        }
        self.output.push(')');
    }

    fn visit_list(&mut self, data: &ListData) {
        self.visit_annotations(&data.annotations);
        self.output.push('[');
        if !data.items.is_empty() {
            let opening_indent = find_cursor_pos(&self.output) - 1;
            let continuation_indent = opening_indent + 1;
            for i in 0..data.items.len() {
                let item = &data.items[i];
                if !item.is_newlines() && last_is_one_of(&self.output, &[',']) {
                    self.output.push(' ');
                }
                if item.is_newlines() && i == data.items.len() - 1 {
                    self.visit_expr(item, opening_indent);
                } else {
                    self.visit_expr(item, continuation_indent);
                }
                if item.is_value() && data.items[(i + 1)..].iter().any(Expr::is_value) {
                    self.output.push(',');
                }
            }
        }
        self.output.push(']');
    }

    fn visit_struct_key(&mut self, data: &NonAnnotatedStringData) {
        if !last_is_one_of(&self.output, &['\n']) {
            self.output.push(' ');
        }
        self.output.push_str(&data.value);
        self.output.push(':');
    }

    fn visit_struct(&mut self, data: &ListData) {
        self.visit_annotations(&data.annotations);

        let empty_continuation = find_cursor_pos(&self.output);
        let key_continuation = empty_continuation + 1;
        let nested_struct_continuation = key_continuation + 3;
        let value_continuation = key_continuation + 3;

        self.output.push('{');
        for i in 0..data.items.len() {
            let value = &data.items[i];
            if value.is_newlines() {
                if let Some(next) = data.items[(i + 1)..]
                    .iter()
                    .find(|item| item.is_value() || item.is_struct_key())
                {
                    if next.is_struct_key() {
                        self.visit_expr(value, key_continuation);
                    } else if next.is_struct() {
                        self.visit_expr(value, nested_struct_continuation);
                    } else {
                        self.visit_expr(value, value_continuation);
                    }
                } else {
                    self.visit_expr(value, empty_continuation);
                }
            } else {
                if last_is_one_of(&self.output, &[':', '/']) || value.is_comment() {
                    self.output.push(' ');
                }
                self.visit_expr(value, 0);
                if value.is_value() && data.items[(i + 1)..].iter().any(Expr::is_value) {
                    self.output.push(',');
                }
            }
        }
        if !last_is_one_of(&self.output, &['{', '}', ' ', '\n']) {
            self.output.push(' ');
        }
        self.output.push('}');
    }
}

fn newline(newline_count: usize, indent: usize) -> String {
    let mut output = repeat('\n', newline_count);
    output.push_str(&repeat(' ', indent));
    output
}

#[derive(Debug, PartialEq, Eq)]
enum IndentType {
    /// (
    ///  1 2 // <-- this indent type
    /// )
    /// Or:
    /// [1,
    ///  2] <-- this indent type
    EndOfOpening,
    /// (foo (bar)
    ///      (baz)) // <-- this indent type
    EndOfOpeningSymbol(usize),
    /// (define (foo)
    ///   (baz)) // <-- this indent type
    /// Or:
    /// ((foo)
    ///   (bar)) // <-- this indent type
    Fixed,

    Undetermined,
}

fn calculate_continuation_indent(config: &FusionConfig, exprs: &[Expr], next_indent: usize) -> usize {
    // Figure out what indentation would be without any config
    let mut indent_type = match exprs.count_items_before_newline() {
        0 => IndentType::EndOfOpening,
        1 => IndentType::Fixed,
        _ => IndentType::Undetermined,
    };

    if let Some(first) = exprs.first() {
        // If the first value is a symbol, then try to
        // override determined indentation with config
        if first.is_symbol() {
            let symbol_value = first.symbol_value().unwrap();
            if indent_type != IndentType::Fixed {
                indent_type = IndentType::EndOfOpeningSymbol(next_indent + symbol_value.len() + 2);
            }
            if config.fixed_indent_symbols.contains(symbol_value) {
                // Symbol configured to always use fixed indent
                indent_type = IndentType::Fixed;
            } else if config.smart_indent_symbols.contains(symbol_value)
                && let IndentType::EndOfOpeningSymbol(_) = indent_type
            {
                let newlines = exprs.count_newlines();
                if newlines > 3 {
                    // Symbol configured to use fixed indent if it's long
                    indent_type = IndentType::Fixed;
                }
            }
        } else if !first.is_sexpr() {
            // Otherwise, if the first is not an s-expression, use end of opening indent
            indent_type = IndentType::EndOfOpening;
        } else {
            indent_type = IndentType::Fixed;
        }
    }

    // Translate indentation type into numbers
    match indent_type {
        IndentType::EndOfOpening => next_indent + 1,
        IndentType::Fixed => next_indent + 2,
        IndentType::EndOfOpeningSymbol(indent) => indent,
        IndentType::Undetermined => unreachable!(),
    }
}
