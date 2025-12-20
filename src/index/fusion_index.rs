// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::index::{Module, ModuleCell, ScriptCell};
use fuusak::error::Error;
use fuusak::file::FusionFile;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const TOP_LEVEL_MODULE_NAME: &str = "/fusion/private/kernel";

pub type FusionIndexCell = Rc<RefCell<FusionIndex>>;

pub struct FusionIndex {
    current_package_path: PathBuf,
    module_paths: Vec<PathBuf>,
    modules: BTreeMap<String, ModuleCell>,
    scripts: BTreeMap<String, ScriptCell>,
}

impl FusionIndex {
    pub fn new(current_package_path: &Path, module_paths: Vec<PathBuf>) -> Result<FusionIndexCell, Error> {
        let result = Rc::new(RefCell::new(FusionIndex {
            current_package_path: current_package_path
                .canonicalize()
                .map_err(|err| err_generic!("failed to canonicalize path: {}", err))?,
            module_paths: module_paths
                .into_iter()
                .map(|path| {
                    path.canonicalize()
                        .map_err(|err| err_generic!("failed to canonicalize path: {}", err))
                })
                .collect::<Result<Vec<PathBuf>, Error>>()?,
            modules: BTreeMap::new(),
            scripts: BTreeMap::new(),
        }));
        println!("Module repository initialized with paths:");
        for path in &result.borrow().module_paths {
            println!("  {}", path.display());
        }
        Ok(result)
    }

    pub fn current_package_path(&self) -> &Path {
        &self.current_package_path
    }

    pub fn get_root_module(&mut self) -> ModuleCell {
        if !self.modules.contains_key(TOP_LEVEL_MODULE_NAME) {
            self.put_module(Module::new(
                TOP_LEVEL_MODULE_NAME.into(),
                TOP_LEVEL_MODULE_NAME.into(),
                FusionFile::empty_file(),
                Vec::new(),
                BTreeMap::new(),
            ));
        }
        self.modules.get(TOP_LEVEL_MODULE_NAME).unwrap().clone()
    }

    pub fn module_iter(&self) -> impl Iterator<Item = &'_ ModuleCell> {
        self.modules.iter().map(|entry| entry.1)
    }

    pub fn script_iter(&self) -> impl Iterator<Item = &'_ ScriptCell> {
        self.scripts.iter().map(|entry| entry.1)
    }

    pub fn get_module(&self, name: &String) -> Option<ModuleCell> {
        self.modules.get(name).cloned()
    }

    pub fn put_module(&mut self, module: ModuleCell) {
        let name = module.borrow().name.clone();
        self.modules.insert(name, module);
    }

    pub fn get_script(&self, name: &String) -> Option<ScriptCell> {
        self.scripts.get(name).cloned()
    }

    pub fn put_script(&mut self, script: ScriptCell) {
        let name = script.borrow().name.clone();
        self.scripts.insert(name, script);
    }

    pub fn find_module_file(&self, module_name: &str) -> Option<PathBuf> {
        let module_file_name = format!(
            "{}.fusion",
            if let Some(stripped) = module_name.strip_prefix('/') {
                stripped
            } else {
                module_name
            }
        );
        for path in &self.module_paths {
            let module_path = path.join(&module_file_name);
            if module_path.exists() {
                return Some(module_path);
            }
        }
        None
    }

    pub fn find_parent_path<'a>(&'a self, file_path: &Path) -> Option<&'a Path> {
        for path in &self.module_paths {
            for ancestor in file_path.ancestors() {
                if ancestor == path {
                    return Some(path);
                }
            }
        }
        None
    }
}

#[allow(clippy::missing_fields_in_debug)]
impl fmt::Debug for FusionIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FusionIndex")
            // omit the module_paths to make testing easier
            .field("modules", &self.modules)
            .field("scripts", &self.scripts)
            .finish()
    }
}
