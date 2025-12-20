// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use fuusak::file::FusionFile;
use std::cell::RefCell;
use std::fmt;
use std::path::Path;
use std::rc::Rc;

pub type ScriptCell = Rc<RefCell<Script>>;

pub struct Script {
    pub name: String,
    pub top_level_modules: Vec<String>,
    pub global_bindings: Vec<String>,
    pub files: Vec<FusionFile>,
}

impl Script {
    pub fn new(
        name: String,
        top_level_modules: Vec<String>,
        global_bindings: Vec<String>,
        files: Vec<FusionFile>,
    ) -> ScriptCell {
        Rc::new(RefCell::new(Script {
            name,
            top_level_modules,
            global_bindings,
            files,
        }))
    }
}

impl fmt::Debug for Script {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Script")
            .field("name", &self.name)
            .field("top_level_modules", &self.top_level_modules)
            .field("global_bindings", &self.global_bindings)
            // omit the AST for files
            .field(
                "files",
                &self
                    .files
                    .iter()
                    .map(|file| file.file_name.as_ref())
                    .collect::<Vec<&Path>>(),
            )
            .finish()
    }
}
