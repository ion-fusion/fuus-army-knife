// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use fuusak::config::FusionConfig;
use fuusak::error::Error;
use std::path::Path;

mod fusion_index;
mod fusion_loader;
mod module;
mod script;

pub use fusion_index::*;
pub use fusion_loader::*;
pub use module::*;
pub use script::*;

pub fn load_index(fusion_config: &FusionConfig, package_path: &Path) -> Result<FusionIndexCell, Error> {
    let mut paths = Vec::new();
    let module_path = package_path.join("fusion/src");
    if module_path.exists() {
        paths.push(module_path);
    }

    let fusion_index = FusionIndex::new(package_path, paths)?;
    let fusion_loader = FusionLoader::new(fusion_config, &fusion_index);
    fusion_loader.load_configured_paths(fusion_config)?;

    Ok(fusion_index)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::diff_util::human_diff_lines;
    use fuusak::config::new_default_config;
    use std::path::PathBuf;

    #[test]
    fn bootstrap_test() {
        let default_config = new_default_config();
        let fusion_index = FusionIndex::new(
            &PathBuf::from("./"),
            vec![PathBuf::from("index_tests/bootstrap/test_files")],
        )
        .unwrap();

        let fusion_loader = FusionLoader::new(&default_config, &fusion_index);
        if let Err(err) = fusion_loader.load_module_file("index_tests/bootstrap/test_files/some_mod.fusion") {
            panic!("\n{err}");
        }

        let expected_repo = include_str!("../../index_tests/bootstrap/expected-repo.txt").trim();
        let actual_repo = format!("{fusion_index:#?}");
        if expected_repo != actual_repo {
            let msg = format!(
                "\nIndexing index_tests/bootstrap/test_files failed:\n{}\n",
                human_diff_lines(expected_repo, actual_repo)
            );
            panic!("{}", msg);
        }
    }
}
