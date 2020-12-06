// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::config::{FusionConfig, FusionPathConfig};
use crate::error::Error;
use crate::file::FusionFile;
use crate::index::{Module, ModuleCell};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::rc::Rc;

pub const TOP_LEVEL_MODULE_NAME: &'static str = "/fusion/private/kernel";

pub type ModuleRepoCell = Rc<RefCell<ModuleRepo>>;

pub struct ModuleRepo {
    current_package_path: PathBuf,
    module_paths: Vec<PathBuf>,
    modules: BTreeMap<String, ModuleCell>,
}

impl ModuleRepo {
    pub fn new(
        current_package_path: PathBuf,
        module_paths: Vec<PathBuf>,
    ) -> Result<ModuleRepoCell, Error> {
        let result = Rc::new(RefCell::new(ModuleRepo {
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
        }));
        println!("Module repository initialized with paths:");
        for path in &result.borrow().module_paths {
            println!("  {:?}", path);
        }
        Ok(result)
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

    pub fn get_module(&self, name: &String) -> Option<ModuleCell> {
        self.modules.get(name).cloned()
    }

    pub fn put_module(&mut self, module: ModuleCell) {
        let name = module.borrow().name.clone();
        self.modules.insert(name, module);
    }

    pub fn resolve_path_config<'a>(
        &self,
        fusion_config: &'a FusionConfig,
        path: &Path,
    ) -> Option<&'a FusionPathConfig> {
        let path = path.canonicalize().ok()?;
        if let Ok(relative) = path.strip_prefix(&self.current_package_path) {
            return fusion_config.resolve_path_config(relative);
        }
        None
    }

    pub fn find_module_file(&self, module_name: &str) -> Option<PathBuf> {
        let module_file_name = format!(
            "{}.fusion",
            if module_name.starts_with("/") {
                &module_name[1..]
            } else {
                &module_name[..]
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

impl fmt::Debug for ModuleRepo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ModuleRepo")
            // omit the module_paths to make testing easier
            .field("modules", &self.modules)
            .finish()
    }
}
