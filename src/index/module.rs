// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::file::FusionFile;
use crate::span::ShortSpan;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt::{self, Debug};
use std::rc::Rc;

pub type ModuleCell = Rc<RefCell<Module>>;

#[derive(new, Debug)]
pub struct Origin {
    pub name: String,
    pub originates_from: ShortSpan,
}

#[derive(new, Debug)]
pub enum RequireType {
    /// For `(require "module")`
    All,
    /// For `(require (only_in ...))`
    Names(Vec<Origin>),
    /// For `(require (rename_in ...))` and `(require (prefix_in ...))`
    Mapped(BTreeMap<String, Origin>),
}

#[derive(new)]
pub struct RequireForm {
    pub module: ModuleCell,
    pub required: RequireType,
}

impl RequireForm {
    pub fn find_origin(&self, name: &String) -> Option<ShortSpan> {
        match &self.required {
            RequireType::All => self.module.borrow().provides.get(name).copied(),
            RequireType::Names(names) => names
                .iter()
                .find(|origin| &origin.name == name)
                .map(|origin| origin.originates_from),
            RequireType::Mapped(mapping) => mapping
                .values()
                .find(|origin| &origin.name == name)
                .map(|origin| origin.originates_from),
        }
    }
}

impl fmt::Debug for RequireForm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RequireForm")
            .field("module", &self.module.borrow().name)
            .field("required", &self.required)
            .finish()
    }
}

pub struct Module {
    pub name: String,
    pub language: String,
    pub file: FusionFile,
    pub requires: Vec<RequireForm>,
    pub provides: BTreeMap<String, ShortSpan>,
}

impl Module {
    pub fn new(
        name: String,
        language: String,
        file: FusionFile,
        requires: Vec<RequireForm>,
        provides: BTreeMap<String, ShortSpan>,
    ) -> ModuleCell {
        Rc::new(RefCell::new(Module {
            name,
            language,
            file,
            requires,
            provides,
        }))
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Module")
            .field("name", &self.name)
            .field("language", &self.language)
            // omit the file since the AST is huge and not particularly useful
            .field("requires", &self.requires)
            .field("provides", &self.provides)
            .finish()
    }
}
