// Copyright Ion Fusion contributors. All Rights Reserved.
#![warn(rust_2018_idioms)]

#[macro_use]
extern crate serde_derive;

mod ast;
mod config;
mod error;
mod lexer;
mod parser;
mod validate;

use crate::ast::Expr;
use crate::config::{FusionConfig, DEFAULT_CONFIG};
use clap::{App, Arg};
use std::path::{Path, PathBuf};
use toml::Value;
use walkdir::WalkDir;

macro_rules! fail {
    ($($arg:expr),*) => {
        {
            eprintln!($($arg,)*);
            ::std::process::exit(1);
        }
    };
}

struct FusionFileContent {
    file_name: PathBuf,
    contents: String,
}

pub struct FusionFile<'i> {
    file_name: &'i Path,
    ast: Vec<Expr<'i>>,
}

fn main() {
    let matches = App::new("Fuus Army Knife (fuusak)")
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
        .get_matches();

    let config_file_name = matches.value_of("config").unwrap_or_else(|| "fuusak.toml");
    let config_contents =
        std::fs::read_to_string(config_file_name).unwrap_or(DEFAULT_CONFIG.into());
    let config = config_contents
        .parse::<Value>()
        .unwrap_or_else(|err| fail!("Failed to parse config file: {}: {}", config_file_name, err));

    let fusion_config = config
        .get("fusion")
        .unwrap_or_else(|| fail!("Missing config 'fusion' top-level in {}", config_file_name))
        .clone()
        .try_into::<FusionConfig>()
        .unwrap_or_else(|err| {
            fail!(
                "Failed to parse 'fusion' top-level config in {}: {}",
                config_file_name,
                err
            )
        });

    let mut fusion_contents: Vec<FusionFileContent> = Vec::new();
    let directory_walker = WalkDir::new(".")
        .follow_links(true)
        .sort_by(|a, b| a.file_name().cmp(b.file_name()));
    for entry in directory_walker {
        let entry = entry.unwrap_or_else(|err| fail!("Failed to read input file: {}", err));
        let path = entry.path();
        let extension = path.extension().and_then(|extension| extension.to_str());
        if !entry.file_type().is_dir() {
            if let Some("fusion") = extension {
                fusion_contents.push(FusionFileContent {
                    file_name: path.to_path_buf(),
                    contents: std::fs::read_to_string(path).unwrap_or_else(|err| {
                        fail!("Failed to load file {}: {}", path.display(), err)
                    }),
                });
            }
        }
    }

    let mut fusion_files = Vec::new();
    for contents in &fusion_contents {
        println!("Examining {:?}...", contents.file_name);
        match parser::parse(&contents.file_name, &contents.contents, &fusion_config) {
            Ok(parse_result) => {
                let file = FusionFile {
                    file_name: &contents.file_name,
                    ast: parse_result,
                };
                let errors = validate::validate(&file.ast);
                if !errors.is_empty() {
                    for error in &errors {
                        eprintln!("  {}\n", error);
                    }
                    println!("  Skipping {:?}...", contents.file_name);
                } else {
                    fusion_files.push(file);
                }
            }
            Err(error) => {
                eprintln!("{}\n", error);
            }
        }
    }

    for file in &fusion_files {
        println!("Formatting {:?}...", file.file_name);
        // TODO
    }
}
