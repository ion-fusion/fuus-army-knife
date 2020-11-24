// Copyright Ion Fusion contributors. All Rights Reserved.
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate derive_new;
#[macro_use]
extern crate serde_derive;

mod ast;
mod config;
mod error;
mod file;
mod format;
mod ist;
mod lexer;
mod parser;
mod span;
mod string_util;
#[cfg(test)]
mod test_util;
mod validate;

use crate::config::{load_config, write_default_config, FusionConfig};
use crate::file::{FusionFile, FusionFileContent};
use clap::{crate_version, App, Arg, SubCommand};
use std::io::Write;
use tempfile::NamedTempFile;
use walkdir::WalkDir;

macro_rules! fail {
    ($($arg:expr),*) => {
        {
            eprintln!($($arg,)*);
            ::std::process::exit(1);
        }
    };
}

fn main() {
    let mut clap_app = configure_clap_app();
    let matches = clap_app.clone().get_matches();

    let config_file_name = matches.value_of("config").unwrap_or_else(|| "fuusak.toml");
    let fusion_config = load_config(config_file_name).unwrap_or_else(|error| fail!("{}", error));

    if let Some(matches) = matches.subcommand_matches("debug-ast") {
        let path = matches.value_of("FILE").unwrap();
        subcommand_debug_ast(&fusion_config, path);
    } else if let Some(matches) = matches.subcommand_matches("debug-ist") {
        let path = matches.value_of("FILE").unwrap();
        subcommand_debug_ist(&fusion_config, path);
    } else if let Some(_) = matches.subcommand_matches("create-config") {
        subcommand_create_config();
    } else if let Some(matches) = matches.subcommand_matches("format") {
        let path = matches.value_of("FILE").unwrap();
        subcommand_format(&fusion_config, path);
    } else if let Some(_) = matches.subcommand_matches("format-all") {
        subcommand_format_all(&fusion_config);
    } else {
        drop(clap_app.print_help());
        println!("\n")
    }
}

fn configure_clap_app<'a, 'b>() -> App<'a, 'b> {
    App::new("Fuus Army Knife (fuusak)")
        .version("0.1")
        .about("A Fusion auto-formatter")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .takes_value(true)
                .help("Specifies the config file to use"),
        )
        .subcommand(
            SubCommand::with_name("debug-ast")
                .about("outputs AST of a Fusion file")
                .arg(Arg::with_name("FILE").required(true).index(1)),
        )
        .subcommand(
            SubCommand::with_name("debug-ist")
                .about("outputs IST of a Fusion file")
                .arg(Arg::with_name("FILE").required(true).index(1)),
        )
        .subcommand(
            SubCommand::with_name("create-config")
                .about("creates a config file for Fuus Army Knife in the current directory"),
        )
        .subcommand(
            SubCommand::with_name("format")
                .about("formats a single file")
                .arg(Arg::with_name("FILE").required(true).index(1)),
        )
        .subcommand(
            SubCommand::with_name("format-all")
                .about("recursively formats all Fusion files from the current directory"),
        )
        .subcommand(SubCommand::with_name("help"))
}

fn subcommand_debug_ast(fusion_config: &FusionConfig, path: &str) {
    let file_content = FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err));
    let file = file_content
        .parse(fusion_config)
        .unwrap_or_else(|err| fail!("{}", err));
    println!("{}", file.debug_ast());
}

fn subcommand_debug_ist(fusion_config: &FusionConfig, path: &str) {
    let file_contents = FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err));
    let file = file_contents
        .parse(fusion_config)
        .unwrap_or_else(|err| fail!("{}", err));
    println!("{}", file.debug_ist());
}

fn subcommand_create_config() {
    write_default_config().unwrap_or_else(|err| fail!("Failed to write default config: {}", err));
}

fn format_file_in_place(fusion_config: &FusionConfig, fusion_file: &FusionFile) {
    let formatted = format::format(fusion_config, &fusion_file.ist);

    // Write formatted to a temp file
    let mut temp_file: NamedTempFile =
        NamedTempFile::new().unwrap_or_else(|err| fail!("Failed to create temp file: {}", err));
    write!(temp_file, "{}", formatted)
        .unwrap_or_else(|err| fail!("Failed to write to temp file: {}", err));

    // Replace original file with temp file via rename
    temp_file
        .into_temp_path()
        .persist(&fusion_file.file_name)
        .unwrap_or_else(|err| {
            fail!(
                "Failed to overwrite {:?} with formatted output: {}",
                fusion_file.file_name,
                err
            )
        });
}

fn subcommand_format(fusion_config: &FusionConfig, path: &str) {
    let file_content = FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err));
    let file = file_content
        .parse(fusion_config)
        .unwrap_or_else(|err| fail!("{}", err));
    format_file_in_place(fusion_config, &file);
}

fn subcommand_format_all(fusion_config: &FusionConfig) {
    let mut fusion_files: Vec<FusionFile> = Vec::new();
    let directory_walker = WalkDir::new(".")
        .follow_links(true)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()));
    for entry in directory_walker {
        let entry = entry.unwrap_or_else(|err| fail!("Failed to read input file: {}", err));
        let path = entry.path();
        let extension = path.extension().and_then(|extension| extension.to_str());
        if !entry.file_type().is_dir() {
            if let Some("fusion") = extension {
                println!("Examining {:?}...", path);
                let contents = FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err));
                let fusion_file = contents
                    .parse(fusion_config)
                    .unwrap_or_else(|err| fail!("{}", err));
                fusion_files.push(fusion_file);
            }
        }
    }

    for file in &fusion_files {
        println!("Formatting {:?}...", file.file_name);
        format_file_in_place(fusion_config, file);
    }
}
