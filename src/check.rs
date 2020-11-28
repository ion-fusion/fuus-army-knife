// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::config::FusionConfig;
use crate::error::Error;
use crate::file::FusionFile;
use crate::ist::{AtomicType, IExpr, ListData};
use crate::span::ShortSpan;
use pest::error::Error as PestError;
use pest::error::ErrorVariant;
use pest::Span;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::rc::Rc;

struct ErrorTracker<'i> {
    file_name: String,
    file_contents: &'i str,
    errors: Vec<Error>,
}

impl<'i> ErrorTracker<'i> {
    fn new(file_name: &'i Path, file_contents: &'i str) -> ErrorTracker<'i> {
        ErrorTracker {
            file_name: format!("{:?}", file_name),
            file_contents,
            errors: Vec::new(),
        }
    }

    fn unbound_ident(&mut self, name: &str, span: ShortSpan) {
        self.custom_error(format!("Unbound identifier {}", name), span);
    }

    fn custom_error<S: Into<String>>(&mut self, message: S, span: ShortSpan) {
        let pest_span = Span::new(self.file_contents, span.start, span.end).unwrap();
        let pest_error = PestError::new_from_span(
            ErrorVariant::<crate::lexer::Rule>::CustomError {
                message: message.into(),
            },
            pest_span,
        )
        .with_path(&self.file_name);
        self.errors
            .push(Error::Generic(format!("{}", pest_error.to_string())));
    }

    fn into_errors(self) -> Vec<Error> {
        self.errors
    }
}

trait Env {
    fn contains(&self, symbol: &str) -> bool;
    fn top_level_define(&self, symbol: &str);
}
trait NewScope {
    fn new_scope(self) -> Self;
}

#[derive(new)]
struct Scope {
    env: Rc<RefCell<dyn Env>>,
    bindings: RefCell<HashSet<String>>,
}
impl Scope {
    fn bind(&self, symbol: &str) {
        self.bindings.borrow_mut().insert(symbol.into());
    }
}
impl NewScope for Rc<RefCell<Scope>> {
    fn new_scope(self) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope::new(
            self.clone(),
            RefCell::new(HashSet::new()),
        )))
    }
}
impl Env for Scope {
    fn contains(&self, symbol: &str) -> bool {
        if self.env.borrow().contains(symbol) {
            return true;
        }
        self.bindings.borrow().contains(symbol)
    }

    fn top_level_define(&self, symbol: &str) {
        self.env.borrow_mut().top_level_define(symbol);
    }
}

struct BindingEnv {
    global: HashSet<String>,
    defines: RefCell<HashSet<String>>,
}
impl BindingEnv {
    pub fn new(config: &FusionConfig) -> BindingEnv {
        BindingEnv {
            global: config.global_bindings.iter().cloned().collect(),
            defines: RefCell::new(HashSet::new()),
        }
    }

    pub fn scope(self) -> Rc<RefCell<Scope>> {
        Rc::new(RefCell::new(Scope::new(
            Rc::new(RefCell::new(self)),
            RefCell::new(HashSet::new()),
        )))
    }
}
impl Env for BindingEnv {
    fn contains(&self, symbol: &str) -> bool {
        if self.global.contains(symbol) {
            return true;
        }
        if self.defines.borrow().contains(symbol) {
            return true;
        }
        false
    }

    fn top_level_define(&self, symbol: &str) {
        self.defines.borrow_mut().insert(symbol.into());
    }
}

pub fn check(config: &FusionConfig, file: &FusionFile) -> Vec<Error> {
    let mut tracker = ErrorTracker::new(&file.file_name, &file.contents);
    check_unbound_ident(&mut tracker, config, file);
    tracker.into_errors()
}

fn check_unbound_ident(tracker: &mut ErrorTracker<'_>, config: &FusionConfig, file: &FusionFile) {
    let scope = BindingEnv::new(config).scope();
    // First, populate all top_level defines
    for expr in &file.ist.expressions {
        // Use temporary error tracker
        check_unbound_expr(
            &mut ErrorTracker::new(&PathBuf::from(""), &file.contents),
            expr,
            scope.clone(),
            false,
        );
    }
    // Now do the unbound check for real
    for expr in &file.ist.expressions {
        check_unbound_expr(tracker, expr, scope.clone(), false);
    }
}

fn check_unbound_expr(
    tracker: &mut ErrorTracker<'_>,
    expr: &IExpr,
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
) {
    use IExpr::*;
    match expr {
        Atomic(data) => match data.typ {
            AtomicType::Symbol => {
                if !quoted && !scope.borrow().contains(&data.value) {
                    tracker.unbound_ident(&data.value, data.span);
                }
            }
            _ => {}
        },
        List(data) => {
            for expr in &data.items {
                check_unbound_expr(tracker, expr, scope.clone(), quoted);
            }
        }
        SExpr(data) => {
            check_unbound_sexpr(tracker, data, scope, quoted);
        }
        Struct(data) => {
            for expr in &data.items {
                check_unbound_expr(tracker, expr, scope.clone(), quoted);
            }
        }
        Clob(_) | CommentBlock(_) | CommentLine(_) | MultilineString(_) | Newlines(_)
        | StructKey(_) => {}
    }
}

fn check_unbound_sexpr(
    tracker: &mut ErrorTracker<'_>,
    sexpr: &ListData,
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
) {
    if let Some(first_value) = sexpr.items.iter().find(|item| item.is_value()) {
        let rest = &sexpr.items[1..];
        if first_value.is_symbol() {
            let function_call = first_value.symbol_value();
            match function_call.as_str() {
                "define" => check_unbound_define(tracker, rest, scope, quoted),
                "lambda" => check_unbound_lambda(tracker, rest, scope, quoted),
                "let" => check_unbound_let(tracker, rest, scope, quoted, false),
                "lets" => check_unbound_let(tracker, rest, scope, quoted, true),
                "module" => check_unbound_module(tracker, rest, scope, quoted),
                "only_in" => check_unbound_only_in(tracker, rest, scope),
                "quasiquote" => check_unbound_quasiquote(tracker, rest, scope),
                "quote" => {}
                "unquote" => check_unbound_unquote(tracker, rest, scope),
                "|" => check_unbound_pipe_lambda(tracker, rest, scope, quoted),
                _ => {
                    if !quoted && !scope.borrow().contains(function_call) {
                        tracker.unbound_ident(function_call, first_value.span());
                    }
                    for item in rest {
                        check_unbound_expr(tracker, item, scope.clone(), quoted);
                    }
                }
            }
        }
    }
}

fn check_unbound_define(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
) {
    if let Some(arg_list) = rest.get(0) {
        let new_scope = scope.new_scope();
        if arg_list.is_symbol() {
            new_scope.borrow().top_level_define(arg_list.symbol_value());
        } else if arg_list.is_sexpr() {
            let arg_list = &arg_list.list_data().items;
            if arg_list.len() >= 1 && arg_list[0].is_symbol() {
                let name = arg_list[0].symbol_value();
                new_scope.borrow().top_level_define(name);
                for item in &arg_list[1..] {
                    if item.is_symbol() {
                        new_scope.borrow().bind(item.symbol_value());
                    }
                }
            }
        }
        if rest.len() > 1 {
            for item in &rest[1..] {
                check_unbound_expr(tracker, item, new_scope.clone(), quoted);
            }
        }
    }
}

fn check_unbound_lambda(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
) {
    if let Some(arg_list) = rest.get(0) {
        let new_scope = scope.new_scope();
        if arg_list.is_symbol() {
            new_scope.borrow_mut().bind(arg_list.symbol_value());
        } else if arg_list.is_sexpr() {
            for item in &arg_list.list_data().items {
                if item.is_symbol() {
                    new_scope.borrow_mut().bind(item.symbol_value());
                }
            }
        }

        if rest.len() > 1 {
            for item in &rest[1..] {
                check_unbound_expr(tracker, item, new_scope.clone(), quoted);
            }
        }
    }
}

fn check_unbound_let(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
    plural: bool,
) {
    let new_scope = scope.clone().new_scope();
    if rest.len() > 1 {
        if rest[0].is_list() {
            for item in &rest[0].list_data().items {
                if item.is_sexpr() {
                    let definition = &item.list_data().items;
                    if definition.len() > 1 && definition[0].is_symbol() {
                        new_scope.borrow_mut().bind(definition[0].symbol_value());
                    }
                    for sub_item in &definition[1..] {
                        if plural {
                            check_unbound_expr(tracker, sub_item, new_scope.clone(), quoted);
                        } else {
                            check_unbound_expr(tracker, sub_item, scope.clone(), quoted);
                        }
                    }
                }
            }
            for item in &rest[1..] {
                check_unbound_expr(tracker, item, new_scope.clone(), quoted);
            }
        }
    }
}

fn check_unbound_module(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
) {
    if rest.len() > 2 {
        for item in &rest[2..] {
            check_unbound_expr(tracker, item, scope.clone(), quoted);
        }
    }
}

fn check_unbound_only_in(
    _tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
) {
    if rest.len() > 1 {
        for item in &rest[1..] {
            if item.is_symbol() {
                scope.borrow_mut().top_level_define(item.symbol_value());
            }
        }
    }
}

fn check_unbound_quasiquote(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
) {
    for item in rest {
        check_unbound_expr(tracker, item, scope.clone(), true);
    }
}

fn check_unbound_unquote(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
) {
    for item in rest {
        check_unbound_expr(tracker, item, scope.clone(), false);
    }
}

fn check_unbound_pipe_lambda(
    tracker: &mut ErrorTracker<'_>,
    rest: &[IExpr],
    scope: Rc<RefCell<Scope>>,
    quoted: bool,
) {
    let new_scope = scope.new_scope();
    let mut arg_list = true;
    for item in rest {
        if arg_list && item.is_symbol() {
            if arg_list && item.symbol_value() == "|" {
                arg_list = false;
            } else {
                new_scope.borrow_mut().bind(item.symbol_value());
            }
        } else if !arg_list {
            check_unbound_expr(tracker, item, new_scope.clone(), quoted);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::new_default_config;
    use crate::file::FusionFileContent;
    use crate::test_util::human_diff_lines;

    macro_rules! test {
        ($input:expr, $output:expr) => {
            let input = include_str!($input);
            let expected_output = include_str!($output).trim();
            let config = new_default_config();
            let file = FusionFileContent::new($input.into(), input.into())
                .parse(&config)
                .unwrap_or_else(|error| panic!("Error: {}", error));
            let errors = check(&config, &file);
            let actual_output = format!(
                "[\n{}\n]",
                errors
                    .into_iter()
                    .map(|e| format!("{}", e))
                    .collect::<Vec<String>>()
                    .join("\n\n---\n\n")
                    .lines()
                    .map(|line| line.trim_end())
                    .fold(String::new(), |l, r| l + r + "\n")
            );
            if expected_output != &actual_output {
                let msg = format!(
                    "\nChecking of {} didn't match expected output in {}:\n{}\n",
                    $input,
                    $output,
                    human_diff_lines(expected_output, actual_output)
                );
                assert!(false, msg);
            }
        };
    }

    #[test]
    fn unbound_identifier() {
        test!(
            "../check_tests/unbound-identifier.fusion",
            "../check_tests/unbound-identifier.errors.txt"
        );
    }
}
