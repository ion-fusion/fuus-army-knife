// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::*;

use crate::config::{FusionConfig, FusionPathMode};
use crate::error::Error;
use crate::file::FusionFile;
use crate::index::{Module, ModuleCell, ModuleRepoCell, Origin, RequireForm, RequireType};
use crate::span::ShortSpan;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(new)]
pub struct ModuleLoader<'i> {
    fusion_config: &'i FusionConfig,
    module_repo: ModuleRepoCell,
}

impl<'i> ModuleLoader<'i> {
    pub fn load_file<P: AsRef<Path>>(&self, file_path: P) -> Result<ModuleCell, Error> {
        let file_path = self.resolve_full_file_path(file_path.as_ref())?;
        let module_name = self.determine_module_name(&file_path)?;
        if let Some(module) = self.module_repo.borrow().get_module(&module_name) {
            return Ok(module);
        }

        let file = FusionFile::load(self.fusion_config, &file_path)
            .map_err(|err| err_generic!("failed to load {:?}: {}", file_path, err))?;

        let module = self.process_file(module_name, file)?;
        if let Some(path_config) = self
            .module_repo
            .borrow()
            .resolve_path_config(self.fusion_config, &file_path)
        {
            match path_config.mode {
                FusionPathMode::Modules => {}
                FusionPathMode::Tests => {
                    self.load_module("/fusion")?;
                }
            }
        }

        self.module_repo.borrow_mut().put_module(module.clone());

        println!("Loaded module: {}", module.borrow().name);
        Ok(module)
    }

    fn load_module(&self, module_name: &str) -> Result<ModuleCell, Error> {
        if module_name == "/fusion/private/kernel" {
            return Ok(self.module_repo.borrow_mut().get_root_module());
        }

        let module_file_name = self
            .module_repo
            .borrow()
            .find_module_file(module_name)
            .ok_or_else(|| {
                err_generic!(
                    "cannot load module named {}: no module file found in module paths",
                    module_name
                )
            })?;
        self.load_file(module_file_name)
    }

    fn resolve_full_file_path(&self, file_path: &Path) -> Result<PathBuf, Error> {
        Ok(if file_path.is_relative() {
            current_dir()?.join(file_path)
        } else {
            file_path.into()
        })
    }

    fn determine_module_name(&self, file_path: &Path) -> Result<String, Error> {
        let module_repo = self.module_repo.borrow();
        let parent_path = module_repo
            .find_parent_path(&file_path)
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
                .map_err(&|err: Error| err.resolve_spanned(&file.file_name, &file.contents))?;
        }

        let (language, requires, provides) = processed.dissolve();
        Ok(Module::new(module_name, language, file, requires, provides))
    }

    fn visit_expr(
        &self,
        processed: &mut ProcessedFile,
        expr: &Expr,
        quoted: bool,
    ) -> Result<(), Error> {
        use Expr::*;
        match expr {
            List(data) => {
                for expr in &data.items {
                    self.visit_expr(processed, expr, quoted)?;
                }
            }
            SExpr(data) => {
                self.visit_sexpr(processed, data, quoted)?;
            }
            Struct(data) => {
                for expr in &data.items {
                    self.visit_expr(processed, expr, quoted)?;
                }
            }
            Atomic(_) | Clob(_) | CommentBlock(_) | CommentLine(_) | MultilineString(_)
            | Newlines(_) | StructKey(_) => {}
        }
        Ok(())
    }

    fn visit_sexpr(
        &self,
        processed: &mut ProcessedFile,
        sexpr: &ListData,
        quoted: bool,
    ) -> Result<(), Error> {
        let mut items = sexpr.item_iter();
        if let Some(first_value) = items.next() {
            if let Some(function_call) = first_value.symbol_value() {
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
                        "define" => self.visit_define(processed, items)?,
                        "define_syntax" => self.visit_defpub(processed, items)?,
                        "defpub" => self.visit_defpub(processed, items)?,
                        "defpub_j" => self.visit_defpub(processed, items)?,
                        "defpub_syntax" => self.visit_defpub(processed, items)?,
                        "module" => self.visit_module(processed, sexpr.span, items)?,
                        "provide" => self.visit_provide(processed, items)?,
                        "quasiquote" => self.visit_quasiquote(processed, items)?,
                        "quote" => {}
                        "require" => self.visit_require(processed, items)?,
                        _ => visit_items(first_value, items)?,
                    }
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
        let _module_name = rest
            .next()
            .ok_or_else(|| err_spanned!(span, "missing module name"))?;
        let language = rest
            .next()
            .map(|expr| {
                expr.string_value()
                    .map(|v| v.as_str())
                    .or(expr.stripped_symbol_value())
            })
            .flatten()
            .ok_or_else(|| err_spanned!(span, "missing module language"))?;
        processed.language = Some(language.to_string());
        self.load_module(language)?;
        for expr in rest {
            self.visit_expr(processed, expr, false)?;
        }
        Ok(())
    }

    fn visit_require(
        &self,
        processed: &mut ProcessedFile,
        rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        for expr in rest {
            match expr {
                Expr::Atomic(data) => match data.typ {
                    AtomicType::QuotedString => {
                        let module = self.load_module(&data.value)?;
                        processed
                            .requires
                            .push(RequireForm::new(module, RequireType::All));
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
            }?
        }
        Ok(())
    }

    fn visit_require_sexpr(
        &self,
        processed: &mut ProcessedFile,
        sexpr: &ListData,
    ) -> Result<(), Error> {
        let mut items = sexpr.item_iter();
        if let Some(first_value) = items.next() {
            if let Some(function_call) = first_value.symbol_value() {
                return match function_call.as_str() {
                    "only_in" => self.visit_require_only_in(processed, sexpr.span, items),
                    "prefix_in" => Err(err_spanned!(
                        first_value.span(),
                        "support for `(require (prefix_in ...))` is not implemented"
                    )),
                    "rename_in" => Err(err_spanned!(
                        first_value.span(),
                        "support for `(require (rename_in ...))` is not implemented"
                    )),
                    _ => Err(err_spanned!(
                        first_value.span(),
                        "invalid argument to require"
                    )),
                };
            }
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
            .map(|expr| expr.string_value())
            .flatten()
            .ok_or_else(|| err_spanned!(span, "missing module name"))?;
        let module = self.load_module(module_name)?;
        processed.requires.push(RequireForm::new(
            module,
            RequireType::Names(
                rest.map(|expr| {
                    let name = expr
                        .stripped_symbol_value()
                        .map(|name| name.to_string())
                        .ok_or_else(|| {
                            err_spanned!(expr.span(), "non-symbol found in require only_in list")
                        });
                    name.map(|value| Origin::new(value, expr.span()))
                })
                .collect::<Result<Vec<Origin>, Error>>()?
                .into_iter()
                .collect(),
            ),
        ));
        Ok(())
    }

    fn visit_provide(
        &self,
        processed: &mut ProcessedFile,
        rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
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
                            "rename_out" => {
                                self.visit_rename_out(processed, first_value.span(), items)?
                            }
                            _ => {
                                return Err(err_spanned!(
                                    provided.span(),
                                    "expected all_defined_out or rename_out"
                                ))
                            }
                        }
                    } else {
                        return Err(err_spanned!(
                            provided.span(),
                            "expected all_defined_out or rename_out"
                        ));
                    }
                } else {
                    return Err(err_spanned!(provided.span(), "unexpected s-expression"));
                }
            }
        }
        Ok(())
    }

    fn visit_rename_out(
        &self,
        processed: &mut ProcessedFile,
        rename_out_span: ShortSpan,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        if let Some(Expr::SExpr(sexpr)) = rest.next() {
            let mut inner_itr = sexpr.item_iter();
            let local_name = inner_itr
                .next()
                .map(|expr| expr.symbol_value())
                .flatten()
                .ok_or_else(|| err_spanned!(rename_out_span, "rename_out requires a local name"))?;
            let provided_name = inner_itr
                .next()
                .map(|expr| expr.symbol_value())
                .flatten()
                .ok_or_else(|| {
                    err_spanned!(rename_out_span, "rename_out requires a provided name")
                })?;

            if let Some(defined) = processed.defined.get(local_name) {
                processed.provides.insert(provided_name.into(), *defined);
            } else if let Some(required) = processed
                .requires
                .iter()
                .map(|require| (*require).find_origin(local_name))
                .find(|origin| origin.is_some())
                .flatten()
            {
                processed.provides.insert(provided_name.into(), required);
            } else {
                unimplemented!()
            }
            Ok(())
        } else {
            return Err(err_spanned!(
                rename_out_span,
                "rename_out expected s-expression"
            ));
        }
    }

    fn visit_define(
        &self,
        processed: &mut ProcessedFile,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        if let Some(arg_list) = rest.next() {
            if let Some(name) = arg_list.symbol_value() {
                processed.defined.insert(name.into(), arg_list.span());
            } else if arg_list.is_sexpr() {
                if let Some(first_arg) = arg_list.list_data().item_iter().next() {
                    if let Some(name) = first_arg.symbol_value() {
                        processed.defined.insert(name.into(), first_arg.span());
                    }
                }
            }
        }
        Ok(())
    }

    fn visit_defpub(
        &self,
        processed: &mut ProcessedFile,
        mut rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
        if let Some(arg_list) = rest.next() {
            if let Some(name) = arg_list.symbol_value() {
                processed.provides.insert(name.into(), arg_list.span());
            } else if arg_list.is_sexpr() {
                if let Some(first_arg) = arg_list.list_data().item_iter().next() {
                    if let Some(name) = first_arg.symbol_value() {
                        processed.provides.insert(name.into(), first_arg.span());
                    }
                }
            }
        }
        Ok(())
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
    fn visit_unquote(
        &self,
        processed: &mut ProcessedFile,
        rest: impl Iterator<Item = &'i Expr>,
    ) -> Result<(), Error> {
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
            self.provides.extend(self.defined.into_iter());
        }
        (
            self.language.unwrap_or("".into()),
            self.requires,
            self.provides,
        )
    }
}
