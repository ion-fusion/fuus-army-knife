// Copyright Ion Fusion contributors. All Rights Reserved.

mod module;
mod module_loader;
mod module_repo;

pub use module::*;
pub use module_loader::*;
pub use module_repo::*;

#[cfg(test)]
mod test {
    use super::*;
    use crate::config::new_default_config;
    use crate::diff_util::human_diff_lines;
    use std::path::PathBuf;

    #[test]
    fn system_test() {
        let default_config = new_default_config();
        let module_repo = ModuleRepo::new(
            PathBuf::from("./"),
            vec![PathBuf::from("index_tests/test_files")],
        )
        .unwrap();

        let module_loader = ModuleLoader::new(&default_config, module_repo.clone());
        if let Err(err) = module_loader.load_file("index_tests/test_files/some_mod.fusion") {
            assert!(false, format!("\n{}", err));
        }

        let expected_repo = include_str!("../../index_tests/expected-repo.txt").trim();
        let actual_repo = format!("{:#?}", module_repo);
        if expected_repo != &actual_repo {
            let msg = format!(
                "\nIndexing index_tests/test_files/some_mod.fusion failed:\n{}\n",
                human_diff_lines(expected_repo, actual_repo)
            );
            assert!(false, msg);
        }
    }
}
