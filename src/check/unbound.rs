// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::{AtomicType, Expr, ListData};
use crate::check::error_tracker::ErrorTracker;
use crate::check::scope::{BindingEnv, Env, NewScope, ScopeRc};
use crate::config::FusionConfig;
use crate::error::Error;
use crate::file::FusionFile;
use crate::index::{FusionIndexCell, ModuleCell};
use crate::span::ShortSpan;
use std::cell::RefCell;
use std::collections::HashSet;

pub enum ModuleOrScript {
    Module(String),
    Script(String),
}

#[derive(new)]
pub struct UnboundChecker<'i> {
    config: &'i FusionConfig,
    index: FusionIndexCell,
}

impl<'i> UnboundChecker<'i> {
    pub fn check(self, resource: ModuleOrScript) -> Vec<Error> {
        let index = self.index.borrow();
        let scope = self.resolve_initial_scope(&resource);

        // Pre-populate definitions
        match resource {
            ModuleOrScript::Module(ref module_name) => {
                let module = index.get_module(module_name).unwrap();
                // Drop the errors for the first pass since we're populating the scope
                drop(
                    UnboundFileChecker::new(self.config, self.index.clone(), &module.borrow().file)
                        .check_scoped(scope.clone()),
                );
            }
            ModuleOrScript::Script(ref script_name) => {
                let script = index.get_script(script_name).unwrap();
                for file in &script.borrow().files {
                    // Drop the errors for the first pass since we're populating the scope
                    drop(
                        UnboundFileChecker::new(self.config, self.index.clone(), file)
                            .check_scoped(scope.clone()),
                    );
                }
            }
        }

        let mut errors = Vec::new();
        match resource {
            ModuleOrScript::Module(ref module_name) => {
                let module_cell = index.get_module(module_name).unwrap();
                let module = module_cell.borrow();
                let file_checker =
                    UnboundFileChecker::new(self.config, self.index.clone(), &module.file);
                errors.extend(file_checker.check_scoped(scope.clone()).into_iter());
            }
            ModuleOrScript::Script(ref script_name) => {
                let script = index.get_script(script_name).unwrap();
                for file in &script.borrow().files {
                    let file_checker =
                        UnboundFileChecker::new(self.config, self.index.clone(), file);
                    errors.extend(file_checker.check_scoped(scope.clone()).into_iter());
                }
            }
        }
        errors
    }

    fn resolve_initial_scope(&self, resource: &ModuleOrScript) -> ScopeRc {
        let index = self.index.borrow();
        BindingEnv::new(RefCell::new(match resource {
            ModuleOrScript::Module(module_name) => {
                let module = index.get_module(module_name).unwrap();
                self.resolve_all_provides(&module)
            }
            ModuleOrScript::Script(script_name) => {
                let script = index.get_script(script_name).unwrap();
                let mut provides = HashSet::new();
                for module_name in &script.borrow().top_level_modules {
                    let top_level_module = index.get_module(module_name).unwrap();
                    provides.extend(self.resolve_all_provides(&top_level_module).into_iter());
                }
                provides.extend(script.borrow().global_bindings.iter().cloned());
                provides
            }
        }))
        .scope()
    }

    fn resolve_all_provides(&self, module: &ModuleCell) -> HashSet<String> {
        let index = self.index.borrow();
        let module = module.borrow();
        let mut provides = HashSet::new();
        provides.extend(module.provides.keys().cloned());

        if module.language != "/fusion/private/kernel" {
            let language_module = index.get_module(&module.language).unwrap();
            provides.extend(self.resolve_all_provides(&language_module).into_iter());
        }
        provides
    }
}

struct UnboundFileChecker<'i> {
    config: &'i FusionConfig,
    index: FusionIndexCell,
    file: &'i FusionFile,
    errors: ErrorTracker<'i>,
}

impl<'i> UnboundFileChecker<'i> {
    fn new(
        config: &'i FusionConfig,
        index: FusionIndexCell,
        file: &'i FusionFile,
    ) -> UnboundFileChecker<'i> {
        UnboundFileChecker {
            config,
            index,
            file,
            errors: ErrorTracker::new(&file.file_name, &file.contents),
        }
    }

    fn check_scoped(mut self, initial_scope: ScopeRc) -> Vec<Error> {
        for expr in &self.file.ast {
            self.check_unbound_expr(expr, initial_scope.clone(), false);
        }
        self.errors.into_errors()
    }

    fn check_unbound_expr(&mut self, expr: &Expr, scope: ScopeRc, quoted: bool) {
        use Expr::*;
        match expr {
            Atomic(data) => match data.typ {
                AtomicType::Symbol => {
                    if !quoted
                        && !scope
                            .borrow()
                            .contains(data.stripped_symbol_value().unwrap())
                    {
                        self.errors.unbound_ident(&data.value, data.span);
                    }
                }
                _ => {}
            },
            List(data) => {
                for expr in &data.items {
                    self.check_unbound_expr(expr, scope.clone(), quoted);
                }
            }
            SExpr(data) => self.check_unbound_sexpr(data, scope, quoted),
            Struct(data) => {
                for expr in &data.items {
                    self.check_unbound_expr(expr, scope.clone(), quoted);
                }
            }
            Clob(_) | CommentBlock(_) | CommentLine(_) | MultilineString(_) | Newlines(_)
            | StructKey(_) => {}
        }
    }

    // TODO: Fix bug where symbols in `(provides ...)` in EnterpriseDigitalBookImpl aren't unbound checked
    fn check_unbound_sexpr(&mut self, sexpr: &ListData, scope: ScopeRc, quoted: bool) {
        let mut items = sexpr.item_iter();
        if let Some(first_value) = items.next() {
            if let Some(function_call) = first_value.stripped_symbol_value() {
                if quoted {
                    match function_call {
                        "unquote" => self.check_unbound_unquote(items, scope),
                        _ => {
                            for item in items {
                                self.check_unbound_expr(item, scope.clone(), quoted);
                            }
                        }
                    }
                } else {
                    match function_call {
                        "define" => self.check_unbound_define(items, scope),
                        "lambda" => self.check_unbound_lambda(items, scope),
                        "let" => self.check_unbound_let(items, scope, false),
                        "lets" => self.check_unbound_let(items, scope, true),
                        "module" => self.check_unbound_module(items, scope),
                        "require" => self.check_require(items, scope),
                        "quasiquote" => self.check_unbound_quasiquote(items, scope),
                        "quote" => {}
                        "|" => self.check_unbound_pipe_lambda(items, scope),
                        _ => {
                            if !quoted && !scope.borrow().contains(function_call) {
                                self.errors.unbound_ident(function_call, first_value.span());
                            }
                        }
                    }
                }
            }
        }
    }

    fn check_require<'a>(&mut self, rest: impl Iterator<Item = &'a Expr>, scope: ScopeRc) {
        for expr in rest {
            match expr {
                Expr::Atomic(data) => match data.typ {
                    AtomicType::QuotedString => {
                        if let Some(module) = self.index.borrow().get_module(&data.value) {
                            for key in module.borrow().provides.keys() {
                                scope.borrow_mut().bind(key.into());
                            }
                        } else {
                            self.errors.custom_error(
                                format!("cannot find module named {}", data.value),
                                data.span,
                            );
                        }
                    }
                    _ => self
                        .errors
                        .custom_error("arguments to require must be string or s-expr", data.span),
                },
                Expr::SExpr(data) => self.check_require_sexpr(data, scope.clone()),
                _ => self
                    .errors
                    .custom_error("arguments to require must be string or s-expr", expr.span()),
            }
        }
    }

    fn check_require_sexpr(&mut self, sexpr: &ListData, scope: ScopeRc) {
        let mut items = sexpr.item_iter();
        if let Some(first_value) = items.next() {
            if let Some(function_call) = first_value.symbol_value() {
                return match function_call.as_str() {
                    "only_in" => self.check_require_only_in(items, sexpr.span, scope),
                    "prefix_in" => self.errors.custom_error(
                        "support for `(require (prefix_in ...))` is not implemented",
                        first_value.span(),
                    ),
                    "rename_in" => self.check_require_rename_in(items, sexpr.span, scope),
                    _ => self
                        .errors
                        .custom_error("invalid argument to require", first_value.span()),
                };
            }
        }
    }

    fn check_require_only_in<'a>(
        &mut self,
        mut rest: impl Iterator<Item = &'a Expr>,
        parent_span: ShortSpan,
        scope: ScopeRc,
    ) {
        let module_name_expr = rest.next();
        if module_name_expr.is_none() {
            self.errors
                .custom_error("expected module name in rename_in", parent_span);
            return;
        }

        for item in rest {
            if let Some(name) = item.stripped_symbol_value() {
                // TODO: Verify the names actually exist in the module
                scope.borrow().bind_top_level(name.into());
            } else {
                self.errors.custom_error("expected symbol", item.span());
            }
        }
    }

    fn check_require_rename_in<'a>(
        &mut self,
        mut rest: impl Iterator<Item = &'a Expr>,
        parent_span: ShortSpan,
        scope: ScopeRc,
    ) {
        let module_name_expr = rest.next();
        if module_name_expr.is_none() {
            self.errors
                .custom_error("expected module name in rename_in", parent_span);
            return;
        }

        let module_name = module_name_expr
            .map(|expr| expr.string_value())
            .flatten()
            .unwrap();
        let pair_expr = rest.next();
        if let Some(list_data) = pair_expr.map(|e| e.sexpr_value()).flatten() {
            let mut items = list_data.item_iter();
            let from_symbol = items.next().map(|e| e.stripped_symbol_value()).flatten();
            let to_symbol = items.next().map(|e| e.stripped_symbol_value()).flatten();
            if from_symbol.is_none() || to_symbol.is_none() {
                self.errors
                    .custom_error("expected two symbols in rename_in pair", list_data.span);
            }
            // TODO: Verify the names actually exist in the module
            scope.borrow_mut().bind(to_symbol.unwrap().into());
        } else if let Some(expr) = pair_expr {
            self.errors
                .custom_error("expected s-expression", expr.span());
        } else {
            self.errors.custom_error(
                "expected s-expression after module name",
                module_name_expr.unwrap().span(),
            );
        }
    }

    fn check_unbound_define<'a>(
        &mut self,
        mut rest: impl Iterator<Item = &'a Expr>,
        scope: ScopeRc,
    ) {
        if let Some(arg_list) = rest.next() {
            let new_scope = scope.new_scope();
            if let Some(name) = arg_list.stripped_symbol_value() {
                new_scope.borrow().bind_top_level(name.into());
            } else if let Some(sexpr_data) = arg_list.sexpr_value() {
                let arg_list = &sexpr_data.items;
                if arg_list.len() >= 1 && arg_list[0].is_symbol() {
                    let name = arg_list[0].stripped_symbol_value().unwrap();
                    new_scope.borrow().bind_top_level(name.into());
                    for item in &arg_list[1..] {
                        if item.is_symbol() {
                            new_scope
                                .borrow()
                                .bind(item.stripped_symbol_value().unwrap().into());
                        }
                    }
                }
            }
            for item in rest {
                self.check_unbound_expr(item, new_scope.clone(), false);
            }
        }
    }

    fn check_unbound_lambda<'a>(
        &mut self,
        mut rest: impl Iterator<Item = &'a Expr>,
        scope: ScopeRc,
    ) {
        if let Some(arg_list) = rest.next() {
            let new_scope = scope.new_scope();
            if let Some(name) = arg_list.stripped_symbol_value() {
                new_scope.borrow().bind(name.into());
            } else if let Some(sexpr_data) = arg_list.sexpr_value() {
                for item in &sexpr_data.items {
                    if item.is_symbol() {
                        new_scope
                            .borrow()
                            .bind(item.stripped_symbol_value().unwrap().into());
                    }
                }
            }

            for item in rest {
                self.check_unbound_expr(item, new_scope.clone(), false);
            }
        }
    }

    fn check_whenlet<'a>(&mut self, mut rest: impl Iterator<Item = &'a Expr>, scope: ScopeRc) {
        let name = rest.next().map(|e| e.stripped_symbol_value()).flatten();
        let condition = rest.next();
        let value = rest.next();

        if name.is_some() && condition.is_some() && value.is_some() {
            let new_scope = scope.clone().new_scope();
            new_scope.borrow().bind(name.unwrap().into());

            self.check_unbound_expr(condition.unwrap(), scope, false);
            self.check_unbound_expr(value.unwrap(), new_scope, false);
        }
    }

    fn check_unbound_let<'a>(
        &mut self,
        mut rest: impl Iterator<Item = &'a Expr>,
        scope: ScopeRc,
        plural: bool,
    ) {
        let new_scope = scope.clone().new_scope();
        if let Some(list_data) = rest.next().map(|e| e.list_value()).flatten() {
            for item in &list_data.items {
                if item.is_sexpr() {
                    let definition = &item.sexpr_value().unwrap().items;
                    if definition.len() > 1 && definition[0].is_symbol() {
                        new_scope
                            .borrow()
                            .bind(definition[0].stripped_symbol_value().unwrap().into());
                    }
                    for sub_item in &definition[1..] {
                        if plural {
                            self.check_unbound_expr(sub_item, new_scope.clone(), false);
                        } else {
                            self.check_unbound_expr(sub_item, scope.clone(), false);
                        }
                    }
                }
            }
            for item in rest {
                self.check_unbound_expr(item, new_scope.clone(), false);
            }
        }
    }

    fn check_unbound_module<'a>(&mut self, rest: impl Iterator<Item = &'a Expr>, scope: ScopeRc) {
        for item in rest.skip(2) {
            self.check_unbound_expr(item, scope.clone(), false);
        }
    }

    fn check_unbound_quasiquote<'a>(
        &mut self,
        rest: impl Iterator<Item = &'a Expr>,
        scope: ScopeRc,
    ) {
        for item in rest {
            self.check_unbound_expr(item, scope.clone(), true);
        }
    }

    fn check_unbound_unquote<'a>(&mut self, rest: impl Iterator<Item = &'a Expr>, scope: ScopeRc) {
        for item in rest {
            self.check_unbound_expr(item, scope.clone(), false);
        }
    }

    fn check_unbound_pipe_lambda<'a>(
        &mut self,
        rest: impl Iterator<Item = &'a Expr>,
        scope: ScopeRc,
    ) {
        let new_scope = scope.new_scope();
        let mut arg_list = true;
        for item in rest {
            if arg_list && item.is_symbol() {
                if arg_list && item.stripped_symbol_value().unwrap() == "|" {
                    arg_list = false;
                } else {
                    new_scope
                        .borrow()
                        .bind(item.stripped_symbol_value().unwrap().into());
                }
            } else if !arg_list {
                self.check_unbound_expr(item, new_scope.clone(), false);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::scope::BindingEnv;
    use crate::config::new_default_config;
    use crate::diff_util::human_diff_lines;
    use crate::index;
    use std::path::PathBuf;

    #[test]
    fn unbound_identifier() {
        let config = new_default_config();
        let package_path = PathBuf::from("check_tests");
        let fusion_index = index::load_index(&config, &package_path).unwrap();

        let scope = BindingEnv::new(RefCell::new(HashSet::new())).scope();
        scope.borrow().bind("+".into());
        scope.borrow().bind("require".into());
        scope.borrow().bind("only_in".into());
        scope.borrow().bind("provide".into());

        // Check unbound_identifier.fusion
        {
            let script_cell = fusion_index
                .borrow()
                .get_script(&"ftst/unbound_identifier.fusion".to_string())
                .unwrap();
            let script = script_cell.borrow();
            let file = &script.files[0];

            drop(
                UnboundFileChecker::new(&config, fusion_index.clone(), &file)
                    .check_scoped(scope.clone()),
            );
            let errors = UnboundFileChecker::new(&config, fusion_index.clone(), &file)
                .check_scoped(scope.clone());
            compare_errors(
                errors,
                "unbound_identifier.errors.txt",
                include_str!("../../check_tests/ftst/unbound_identifier.errors.txt"),
            );
        }

        // Check some_other_module.fusion
        {
            let module_cell = fusion_index
                .borrow()
                .get_module(&"/some_other_module".to_string())
                .unwrap();
            let module = module_cell.borrow();

            drop(
                UnboundFileChecker::new(&config, fusion_index.clone(), &module.file)
                    .check_scoped(scope.clone()),
            );
            let errors = UnboundFileChecker::new(&config, fusion_index.clone(), &module.file)
                .check_scoped(scope);
            compare_errors(
                errors,
                "some_other_module.errors.txt",
                include_str!("../../check_tests/fusion/src/some_other_module.errors.txt"),
            );
        }
    }

    fn compare_errors(errors: Vec<Error>, file_name: &str, expected_output: &str) {
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
        if expected_output.trim() != &actual_output {
            let msg = format!(
                "\nChecking of unbound_identifier.fusion didn't match expected output in {}:\n{}\n",
                file_name,
                human_diff_lines(expected_output.trim(), actual_output)
            );
            assert!(false, msg);
        }
    }
}
