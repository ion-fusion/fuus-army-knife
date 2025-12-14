// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::ast::{AtomicType, Expr, ListData};
use crate::config::FusionConfig;
use crate::error::Error;
use crate::file::{FusionFile, find_files};
use crate::index::{FusionIndexCell, Module, ModuleCell, Origin, RequireForm, RequireType, Script, ScriptCell};
use crate::span::ShortSpan;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::string::ToString;

pub struct FusionLoader<'i> {
    config: &'i FusionConfig,
    index: FusionIndexCell,
    current_package_path: PathBuf,
}

impl<'i> FusionLoader<'i> {
    pub fn new(config: &'i FusionConfig, fusion_index: &FusionIndexCell) -> FusionLoader<'i> {
        FusionLoader {
            config,
            index: fusion_index.clone(),
            // Retain a copy of the current_package_path so that we can load modules while
            // using it without running into runtime memory ownership issues.
            current_package_path: fusion_index.borrow().current_package_path().into(),
        }
    }

    pub fn load_configured_paths(&self, _config: &FusionConfig) -> Result<(), Error> {
        // Load modules
        let module_path = self.current_package_path.join("fusion/src");
        if module_path.exists() {
            let fusion_file_paths = find_files(module_path, ".fusion")?;
            for file_path in &fusion_file_paths {
                self.load_module_file(file_path)?;
            }
        }
        // Load tests
        let test_path = self.current_package_path.join("ftst");
        if test_path.exists() {
            let fusion_file_paths = find_files(test_path, ".fusion")?;
            for file_path in &fusion_file_paths {
                let relative_path = file_path.strip_prefix(&self.current_package_path).unwrap();
                let test_name = relative_path.to_string_lossy();
                self.load_script(
                    test_name.to_string(),
                    vec!["/fusion".into()],
                    Vec::new(),
                    vec![relative_path.to_path_buf()],
                )?;
                println!("Loaded test: {test_name}");
            }
        }
        Ok(())
    }

    pub fn load_module_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ModuleCell, Error> {
        let file_path = self.resolve_full_file_path(file_path.as_ref());
        let module_name = self.determine_module_name(&file_path)?;
        if let Some(module) = self.index.borrow().get_module(&module_name) {
            return Ok(module);
        }

        self.reload_module_file(module_name, file_path.as_ref())
    }

    pub fn reload_module_file(&self, module_name: String, file_path: &Path) -> Result<ModuleCell, Error> {
        let file = FusionFile::load(self.config, file_path)
            .map_err(|err| err_generic!("failed to load {:?}: {}", file_path, err))?;

        let module = self.process_file(module_name, file)?;
        self.index.borrow_mut().put_module(module.clone());

        println!("Loaded module: {}", module.borrow().name);
        Ok(module)
    }

    pub fn load_module(&self, module_name: &str) -> Result<ModuleCell, Error> {
        if module_name == "/fusion/private/kernel" {
            return Ok(self.index.borrow_mut().get_root_module());
        }

        let module_file_name = self.index.borrow().find_module_file(module_name).ok_or_else(|| {
            err_generic!(
                "cannot load module named {}: no module file found in module paths",
                module_name
            )
        })?;
        self.load_module_file(module_file_name)
    }

    pub fn load_script(
        &self,
        name: String,
        top_level_modules: Vec<String>,
        global_bindings: Vec<String>,
        file_names: Vec<PathBuf>,
    ) -> Result<ScriptCell, Error> {
        for top_level in &top_level_modules {
            self.load_module(top_level)?;
        }

        let files = file_names
            .into_iter()
            .map(|file_name| {
                FusionFile::load(self.config, self.resolve_full_file_path(&file_name)).map(|mut file| {
                    // Go back to the "relative to Config" file name for tests
                    file.file_name = file_name;
                    file
                })
            })
            .collect::<Result<Vec<FusionFile>, Error>>()?;

        for file in &files {
            let mut processed = ProcessedFile::new();
            for expr in &file.ast {
                self.visit_expr(&mut processed, expr, false)
                    .map_err(|err: Error| err.resolve_spanned(&file.file_name, &file.contents))?;
            }
            drop(processed);
        }

        let script = Script::new(name, top_level_modules, global_bindings, files);
        self.index.borrow_mut().put_script(script.clone());
        Ok(script)
    }

    fn resolve_full_file_path<'a>(&self, file_path: &'a Path) -> Cow<'a, Path> {
        if file_path.is_relative() {
            Cow::Owned(self.current_package_path.join(file_path))
        } else {
            Cow::Borrowed(file_path)
        }
    }

    fn determine_module_name(&self, file_path: &Path) -> Result<String, Error> {
        let module_repo = self.index.borrow();
        let parent_path = module_repo
            .find_parent_path(file_path)
            .ok_or_else(|| err_generic!("failed to find parent path of {:?}", file_path))?;
        let relative_module_path = file_path
            .strip_prefix(parent_path)
            .unwrap()
            .as_os_str()
            .to_string_lossy();
        Ok(format!(
            "/{}",
            &relative_module_path[0..relative_module_path.find(".fusion").unwrap()]
        ))
    }

    fn process_file(&self, module_name: String, file: FusionFile) -> Result<ModuleCell, Error> {
        let mut processed = ProcessedFile::new();

        for expr in &file.ast {
            self.visit_expr(&mut processed, expr, false)
                .map_err(|err: Error| err.resolve_spanned(&file.file_name, &file.contents))?;
        }

        let (language, requires, provides) = processed.dissolve();
        Ok(Module::new(module_name, language, file, requires, provides))
    }

    fn visit_expr(&self, processed: &mut ProcessedFile, expr: &Expr, quoted: bool) -> Result<(), Error> {
        match expr {
            Expr::SExpr(data) => {
                self.visit_sexpr(processed, data, quoted)?;
            }
            Expr::List(data) | Expr::Struct(data) => {
                for expr in &data.items {
                    self.visit_expr(processed, expr, quoted)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn visit_sexpr(&self, processed: &mut ProcessedFile, sexpr: &'i ListData, quoted: bool) -> Result<(), Error> {
        let mut items = sexpr.item_iter();
        if let Some(first_value) = items.next()
            && let Some(function_call) = first_value.symbol_value()
        {
            let visit_items = &mut |first, rest| -> Result<(), Error> {
                self.visit_expr(processed, first, quoted)?;
                for item in rest {
                    self.visit_expr(processed, item, quoted)?;
                }
                Ok(())
            };

            if quoted {
                match function_call.as_str() {
                    "unquote" => self.visit_unquote(processed, items)?,
                    _ => visit_items(first_value, items)?,
                }
            } else {
                match function_call.as_str() {
                    "define" => Self::visit_define(processed, items),
                    "define_syntax" | "defpub" | "defpub_j" | "defpub_syntax" => Self::visit_defpub(processed, items),
                    "module" => self.visit_module(processed, sexpr.span, items)?,
                    "provide" => Self::visit_provide(processed, items)?,
                    "quasiquote" => self.visit_quasiquote(processed, items)?,
                    "quote" => {}
                    "require" => self.visit_require(processed, items)?,
                    _ => visit_items(first_value, items)?,
                }
            }
        }
        Ok(())
    }

    fn visit_module(
        &self,
        processed: &mut ProcessedFile,
        span: ShortSpan,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        let _module_name = rest.next().ok_or_else(|| err_spanned!(span, "missing module name"))?;
        let language = rest
            .next()
            .and_then(|expr| expr.string_value().map(String::as_str).or(expr.stripped_symbol_value()))
            .ok_or_else(|| err_spanned!(span, "missing module language"))?;
        processed.language = Some(language.to_string());
        self.load_module(language)?;
        for expr in rest {
            self.visit_expr(processed, expr, false)?;
        }
        Ok(())
    }

    fn visit_require(&self, processed: &mut ProcessedFile, rest: impl Iterator<Item = &'i Expr>) -> Result<(), Error> {
        for expr in rest {
            match expr {
                Expr::Atomic(data) => match data.typ {
                    AtomicType::QuotedString => {
                        let module = self.load_module(&data.value)?;
                        processed.requires.push(RequireForm::new(module, RequireType::All));
                        Ok(())
                    }
                    _ => Err(err_spanned!(
                        data.span,
                        "argument 0 to require must be string or s-expr"
                    )),
                },
                Expr::SExpr(data) => self.visit_require_sexpr(processed, data),
                _ => Err(err_spanned!(
                    expr.span(),
                    "argument 0 to require must be string or s-expr"
                )),
            }?;
        }
        Ok(())
    }

    fn visit_require_sexpr(&self, processed: &mut ProcessedFile, sexpr: &ListData) -> Result<(), Error> {
        let mut items = sexpr.item_iter();
        if let Some(first_value) = items.next()
            && let Some(function_call) = first_value.symbol_value()
        {
            return match function_call.as_str() {
                "only_in" => self.visit_require_only_in(processed, sexpr.span, items),
                "prefix_in" => Err(err_spanned!(
                    first_value.span(),
                    "support for `(require (prefix_in ...))` is not implemented"
                )),
                "rename_in" => self.visit_require_rename_in(processed, sexpr.span, items),
                _ => Err(err_spanned!(first_value.span(), "invalid argument to require")),
            };
        }
        Ok(())
    }

    fn visit_require_only_in(
        &self,
        processed: &mut ProcessedFile,
        span: ShortSpan,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        let module_name = rest
            .next()
            .and_then(|expr| expr.string_value())
            .ok_or_else(|| err_spanned!(span, "missing module name"))?;
        let module = self.load_module(module_name)?;
        processed.requires.push(RequireForm::new(
            module,
            RequireType::Names(
                rest.map(|expr| {
                    let name = expr
                        .stripped_symbol_value()
                        .map(ToString::to_string)
                        .ok_or_else(|| err_spanned!(expr.span(), "non-symbol found in require only_in list"));
                    name.map(|value| Origin::new(value, expr.span()))
                })
                .collect::<Result<Vec<Origin>, Error>>()?
                .into_iter()
                .collect(),
            ),
        ));
        Ok(())
    }

    fn visit_require_rename_in(
        &self,
        processed: &mut ProcessedFile,
        span: ShortSpan,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        let module_name = rest
            .next()
            .and_then(|expr| expr.string_value())
            .ok_or_else(|| err_spanned!(span, "missing module name"))?;
        let module = self.load_module(module_name)?;
        processed.requires.push(RequireForm::new(
            module,
            RequireType::Mapped(
                rest.map(|expr| {
                    let pair = expr
                        .sexpr_value()
                        .map(|sexpr| {
                            sexpr
                                .item_iter()
                                .map(|expr| {
                                    expr.stripped_symbol_value()
                                        .map(ToString::to_string)
                                        .ok_or_else(|| err_spanned!(expr.span(), "expected string"))
                                })
                                .collect::<Result<Vec<String>, Error>>()
                        })
                        .ok_or_else(|| err_spanned!(expr.span(), "expected s-expression"))??;
                    if pair.len() == 2 {
                        Ok((pair[0].clone(), Origin::new(pair[1].clone(), expr.span())))
                    } else {
                        Err(err_spanned!(expr.span(), "invalid rename_in mapping"))
                    }
                })
                .collect::<Result<BTreeMap<String, Origin>, Error>>()?
                .into_iter()
                .collect(),
            ),
        ));
        Ok(())
    }

    fn visit_provide(processed: &mut ProcessedFile, rest: impl Iterator<Item = &'i Expr>) -> Result<(), Error> {
        for provided in rest {
            if let Some(name) = provided.stripped_symbol_value() {
                processed.provides.insert(name.into(), provided.span());
            } else if let Expr::SExpr(sexpr) = provided {
                let mut items = sexpr.item_iter();
                if let Some(first_value) = items.next() {
                    if let Some(function_call) = first_value.symbol_value() {
                        match function_call.as_str() {
                            "all_defined_out" => {
                                processed.all_defined_out = true;
                            }
                            "rename_out" => Self::visit_rename_out(processed, first_value.span(), items)?,
                            _ => return Err(err_spanned!(provided.span(), "expected all_defined_out or rename_out")),
                        }
                    } else {
                        return Err(err_spanned!(provided.span(), "expected all_defined_out or rename_out"));
                    }
                } else {
                    return Err(err_spanned!(provided.span(), "unexpected s-expression"));
                }
            }
        }
        Ok(())
    }

    fn visit_rename_out(
        processed: &mut ProcessedFile,
        rename_out_span: ShortSpan,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        if let Some(Expr::SExpr(sexpr)) = rest.next() {
            let mut inner_itr = sexpr.item_iter();
            let local_name = inner_itr
                .next()
                .and_then(|expr| expr.symbol_value())
                .ok_or_else(|| err_spanned!(rename_out_span, "rename_out requires a local name"))?;
            let provided_name = inner_itr
                .next()
                .and_then(|expr| expr.symbol_value())
                .ok_or_else(|| err_spanned!(rename_out_span, "rename_out requires a provided name"))?;

            if let Some(defined) = processed.defined.get(local_name) {
                processed.provides.insert(provided_name.into(), *defined);
            } else if let Some(required) = processed
                .requires
                .iter()
                .map(|require| (*require).find_origin(local_name))
                .find(Option::is_some)
                .flatten()
            {
                processed.provides.insert(provided_name.into(), required);
            } else {
                unimplemented!()
            }
            Ok(())
        } else {
            Err(err_spanned!(rename_out_span, "rename_out expected s-expression"))
        }
    }

    fn visit_define(processed: &mut ProcessedFile, mut rest: impl Iterator<Item = &'i Expr>) {
        if let Some(arg_list) = rest.next() {
            if let Some(name) = arg_list.symbol_value() {
                processed.defined.insert(name.into(), arg_list.span());
            } else if let Some(sexpr_value) = arg_list.sexpr_value()
                && let Some(first_arg) = sexpr_value.item_iter().next()
                && let Some(name) = first_arg.symbol_value()
            {
                processed.defined.insert(name.into(), first_arg.span());
            }
        }
    }

    fn visit_defpub(processed: &mut ProcessedFile, mut rest: impl Iterator<Item = &'i Expr>) {
        if let Some(arg_list) = rest.next() {
            if let Some(name) = arg_list.symbol_value() {
                processed.provides.insert(name.into(), arg_list.span());
            } else if let Some(sexpr_value) = arg_list.sexpr_value()
                && let Some(first_arg) = sexpr_value.item_iter().next()
                && let Some(name) = first_arg.symbol_value()
            {
                processed.provides.insert(name.into(), first_arg.span());
            }
        }
    }

    fn visit_quasiquote(
        &self,
        processed: &mut ProcessedFile,
        rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        for item in rest {
            self.visit_expr(processed, item, true)?;
        }
        Ok(())
    }
    fn visit_unquote(&self, processed: &mut ProcessedFile, rest: impl Iterator<Item = &'i Expr>) -> Result<(), Error> {
        for item in rest {
            self.visit_expr(processed, item, false)?;
        }
        Ok(())
    }
}

struct ProcessedFile {
    language: Option<String>,
    all_defined_out: bool,
    defined: BTreeMap<String, ShortSpan>,
    requires: Vec<RequireForm>,
    provides: BTreeMap<String, ShortSpan>,
}

impl ProcessedFile {
    fn new() -> ProcessedFile {
        ProcessedFile {
            language: None,
            all_defined_out: false,
            defined: BTreeMap::new(),
            requires: Vec::new(),
            provides: BTreeMap::new(),
        }
    }

    fn dissolve(mut self) -> (String, Vec<RequireForm>, BTreeMap<String, ShortSpan>) {
        if self.all_defined_out {
            self.provides.extend(self.defined);
        }
        (self.language.unwrap_or_default(), self.requires, self.provides)
    }
}
