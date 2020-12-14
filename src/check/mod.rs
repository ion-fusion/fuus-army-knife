// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::check::unbound::{ModuleOrScript, UnboundChecker};
use crate::config::FusionConfig;
use crate::error::Error;
use crate::index::{self, FusionIndexCell, FusionLoader};
use colorful::{Color, Colorful};
use notify::DebouncedEvent::{Remove, Rename, Write};
use notify::{watcher, RecommendedWatcher, RecursiveMode, Watcher};
use rand::random;
use std::collections::{HashMap, HashSet};
use std::env;
use std::path::{Path, PathBuf};
use std::sync::mpsc::channel;
use std::time::Duration;

mod error_tracker;
mod scope;
mod unbound;

pub fn check_correctness_watch(fusion_config: &FusionConfig) -> Result<bool, Error> {
    // Start by indexing the entire package
    let current_package_path = env::current_dir()
        .map_err(|err| err_generic!("failed to determine current working directory: {}", err))?;
    let fusion_index = index::load_index(fusion_config, &current_package_path)?;

    // Now set up a file watcher on the directories relevant to this package
    let watch_paths = build_watch_paths(&current_package_path, fusion_config);
    let file_references = build_references(&current_package_path, &fusion_index, &watch_paths);

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_millis(50))
        .map_err(|err| err_generic!("Failed to create file watch: {}", err))?;
    for path in &watch_paths {
        watch_path(&mut watcher, &path)?;
    }

    // Watch for file system changes
    println!("Successfully indexed all resources used by this package. Watching for filesystem changes now...\n");
    loop {
        match rx.recv() {
            Ok(Write(path)) => {
                // If the file is referenced by the index and is relevant to this package
                if let Some(reference) = file_references.get(&path) {
                    let fusion_loader = FusionLoader::new(fusion_config, fusion_index.clone());
                    match reference {
                        // If it's a module file, reload it
                        Reference::Module(name) => {
                            let errors = reload_module(
                                &current_package_path,
                                fusion_config,
                                &fusion_index,
                                &fusion_loader,
                                name,
                                &path,
                            );
                            if errors.is_empty() {
                                youre_awesome();
                            } else {
                                for error in errors {
                                    println!("{}\n", error);
                                }
                            }
                        }
                        // If it's referenced by a bunch of scripts, reload all of them
                        Reference::Scripts(names) => {
                            reload_scripts(&fusion_config, &fusion_index, &fusion_loader, names)
                        }
                    }
                } else {
                    // TODO: Remove this debug print once confident in correctness
                    println!("(debug) References:\n {:#?}", file_references);
                    println!("Ignoring change to {:?}", path);
                }
            }
            Ok(Remove(_path)) => {
                println!("Proper handling of file deletions is unimplemented. Restarting check-correctness-watch...");
                return Ok(true);
            }
            Ok(Rename(_old_name, _new_name)) => {
                println!("Proper handling of file renames is unimplemented. Restarting check-correctness-watch...");
                return Ok(true);
            }
            Ok(_) => {}
            Err(err) => {
                return Err(err_generic!(
                    "Failed to listen on file system notifications: {}",
                    err
                ))
            }
        }
    }
}

fn reload_module(
    package_path: &Path,
    config: &FusionConfig,
    fusion_index: &FusionIndexCell,
    fusion_loader: &FusionLoader<'_>,
    name: &String,
    path: &Path,
) -> Vec<Error> {
    let mut errors = Vec::new();
    match fusion_loader.reload_module_file(name.into(), &path) {
        Ok(_) => {
            errors.extend(
                UnboundChecker::new(config, fusion_index.clone())
                    .check(ModuleOrScript::Module(name.into()))
                    .into_iter(),
            );
            if errors.is_empty() {
                let impacted_modules: Vec<String> = fusion_index
                    .borrow()
                    .module_iter()
                    .filter(|module| {
                        module.borrow().language == *name
                            || module
                                .borrow()
                                .requires
                                .iter()
                                .any(|require| require.module.borrow().name == *name)
                    })
                    .map(|module| module.borrow().name.clone())
                    .collect();
                for module_name in impacted_modules {
                    let module_path =
                        package_path.join(format!("fusion/src{}.fusion", module_name));
                    errors.extend(
                        reload_module(
                            package_path,
                            config,
                            fusion_index,
                            fusion_loader,
                            &module_name,
                            &module_path,
                        )
                        .into_iter(),
                    );
                }
                // TODO: Reload all the scripts that use the reloaded module
            }
        }
        Err(err) => error_occurred(&package_path, &path, err),
    }
    errors
}

fn reload_scripts(
    config: &FusionConfig,
    fusion_index: &FusionIndexCell,
    fusion_loader: &FusionLoader<'_>,
    names: &HashSet<String>,
) {
    let mut success = true;
    for script_name in names.iter() {
        let (modules, globals, file_names) = {
            let fusion_index = fusion_index.borrow();
            let script_cell = fusion_index.get_script(&script_name).unwrap();
            let script = script_cell.borrow();
            (
                script.top_level_modules.clone(),
                script.global_bindings.clone(),
                script
                    .files
                    .iter()
                    .map(|f| f.file_name.clone())
                    .collect::<Vec<PathBuf>>(),
            )
        };
        match fusion_loader.load_script(script_name.into(), modules, globals, file_names) {
            Ok(_) => {}
            Err(err) => {
                println!("{}\n{}\n", "\nError:".color(Color::Red), err);
                success = false;
                break;
            }
        }
        let errors = UnboundChecker::new(config, fusion_index.clone())
            .check(ModuleOrScript::Script(script_name.into()));
        if errors.is_empty() {
            println!("Reloaded {}.", script_name);
        } else {
            success = false;
            for error in errors {
                println!("{}\n", error);
            }
        }
    }
    if success {
        youre_awesome();
    }
}

const AWESOME_MESSAGES: &'static [&'static str] = &[
    "You're awesome!",
    "Wow, that just worked!",
    "Fantastic. First try?",
    "You rock!",
    "You just wrote some Fusion!",
    "Congrats.",
    "Looks beautiful!",
    "Looks great!",
    "Neat.",
    "BOOM!",
    "Zoomies.",
    "Time for CR, right?",
    "<3",
];

fn youre_awesome() {
    let rindex: usize = random();
    let num: u8 = random();
    let message = AWESOME_MESSAGES[rindex % AWESOME_MESSAGES.len()];
    if message.len() < 15 || num > 32 {
        println!("{}", format!("\n{}", message).color(Color::Blue));
    } else {
        use colorful::HSL;
        println!(
            "{}",
            format!("\n{}", message)
                .gradient_with_color(HSL::new(0.0, 1.0, 0.5), HSL::new(0.833, 1.0, 0.5))
        );
    }
}

fn error_occurred(package_path: &Path, path: &Path, err: Error) {
    let relative = path.strip_prefix(package_path).unwrap();
    println!(
        "{}\n{}\n",
        format!("\nError in {:?}:", relative).color(Color::Red),
        err
    );
}

fn build_watch_paths(package_path: &Path, _config: &FusionConfig) -> Vec<PathBuf> {
    let paths = vec!["fusion/src", "ftst"];
    paths
        .into_iter()
        .map(|path| package_path.join(path))
        .collect()
}

#[derive(Debug)]
enum Reference {
    Module(String),
    Scripts(HashSet<String>),
}

fn build_references(
    package_path: &Path,
    fusion_index: &FusionIndexCell,
    watch_paths: &Vec<PathBuf>,
) -> HashMap<PathBuf, Reference> {
    let mut references = HashMap::new();

    for module in fusion_index.borrow().module_iter() {
        let module = module.borrow();
        let file_name = package_path.join(format!("fusion/src{}.fusion", module.name));
        if file_name.exists() {
            assert!(!references.contains_key(&file_name));
            references.insert(file_name, Reference::Module(module.name.clone()));
        }
    }
    for script in fusion_index.borrow().script_iter() {
        let script = script.borrow();
        for file in &script.files {
            let file_name = package_path.join(&file.file_name);
            if watch_paths
                .iter()
                .any(|path| file_name.strip_prefix(path).is_ok())
            {
                if references.contains_key(&file_name) {
                    if let Some(Reference::Scripts(ref mut names)) = references.get_mut(&file_name)
                    {
                        names.insert(script.name.clone());
                    }
                } else {
                    let mut names = HashSet::new();
                    names.insert(script.name.clone());
                    references.insert(file_name, Reference::Scripts(names));
                }
            }
        }
    }

    references
}

fn watch_path(watcher: &mut RecommendedWatcher, path: &Path) -> Result<(), Error> {
    watcher
        .watch(path, RecursiveMode::Recursive)
        .map_err(|err| err_generic!("Failed to watch {:?}: {}", path, err))
}
