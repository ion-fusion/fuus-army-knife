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
mod ist;
mod lexer;
mod parser;
mod validate;

use crate::config::{load_config, FusionConfig};
use crate::file::FusionFileContent;
use clap::{crate_version, App, Arg, SubCommand};
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
    let file_contents = FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err));
    let file = file_contents
        .parse(fusion_config)
        .unwrap_or_else(|err| fail!("{}", err));
    println!("{:#?}", file.ast);
}

fn subcommand_debug_ist(fusion_config: &FusionConfig, path: &str) {
    let file_contents = FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err));
    let file = file_contents
        .parse(fusion_config)
        .unwrap_or_else(|err| fail!("{}", err));
    println!("{:#?}", file.ist.expressions);
}

fn subcommand_format(_fusion_config: &FusionConfig, _path: &str) {
    unimplemented!()
}

fn subcommand_format_all(fusion_config: &FusionConfig) {
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
                fusion_contents
                    .push(FusionFileContent::load(path).unwrap_or_else(|err| fail!("{}", err)));
            }
        }
    }

    let mut fusion_files = Vec::new();
    for contents in &fusion_contents {
        println!("Examining {:?}...", contents.file_name);
        match contents.parse(fusion_config) {
            Ok(file) => {
                fusion_files.push(file);
            }
            Err(error) => {
                fail!("{}", error);
            }
        }
    }

    for file in &fusion_files {
        println!("Validating {:?}...", file.file_name);
        let errors = validate::validate(&file.ast);
        if !errors.is_empty() {
            for error in &errors {
                eprintln!("  {}\n", error);
            }
            println!("  Skipping {:?}...", file.file_name);
        }

        println!("Formatting {:?}...", file.file_name);
        // TODO
    }
}
