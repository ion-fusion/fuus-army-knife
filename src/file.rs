// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::ast::Expr;
use crate::config::FusionConfig;
use crate::error::Error;
use crate::parser;
use derive_new::new;
use regex::{Captures, Regex};
use std::fs::{FileType, read_to_string};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

#[derive(new, Debug)]
pub struct FusionFile {
    pub file_name: PathBuf,
    pub contents: String,
    pub ast: Vec<Expr>,
}

impl FusionFile {
    pub fn empty_file() -> FusionFile {
        FusionFile {
            file_name: PathBuf::from("empty"),
            contents: String::new(),
            ast: Vec::new(),
        }
    }

    pub fn load<P: AsRef<Path>>(fusion_config: &FusionConfig, path: P) -> Result<FusionFile, Error> {
        FusionFileContent::load(path)?.parse(fusion_config)
    }

    pub fn recursively_load_directory<P: AsRef<Path>>(
        fusion_config: &FusionConfig,
        path: P,
    ) -> Result<Vec<FusionFile>, Error> {
        let fusion_file_paths = find_files(path, ".fusion")?;
        let mut fusion_files = Vec::new();
        for fusion_file_path in &fusion_file_paths {
            let contents = FusionFileContent::load(fusion_file_path).map_err(|err| err_generic!("{}", err))?;
            let fusion_file = contents.parse(fusion_config).map_err(|err| err_generic!("{}", err))?;
            fusion_files.push(fusion_file);
        }
        Ok(fusion_files)
    }

    pub fn debug_ast(&self) -> String {
        let debug_view = format!("{:#?}", self.ast);
        replace_spans(&self.contents, &debug_view)
    }
}

/// Include the "." in `desired_extension`
pub fn find_files<P: AsRef<Path>>(path: P, desired_extension: &str) -> Result<Vec<PathBuf>, Error> {
    let mut fusion_files: Vec<PathBuf> = Vec::new();
    let directory_walker = ignore::WalkBuilder::new(path.as_ref())
        .follow_links(true)
        .sort_by_file_path(Ord::cmp)
        .build();
    for entry in directory_walker {
        let entry = entry.map_err(|err| err_generic!("Failed to read input file: {}", err))?;
        let path = entry.path();
        if !entry.file_type().as_ref().is_none_or(FileType::is_dir)
            && path.as_os_str().to_string_lossy().ends_with(desired_extension)
        {
            fusion_files.push(path.into());
        }
    }
    Ok(fusion_files)
}

fn replace_spans(file_content: &str, debug_view: &str) -> String {
    let span_finder = Regex::new(r"\[Span\((\d+)->(\d+)\)\]").unwrap();
    span_finder
        .replace_all(debug_view, |caps: &Captures<'_>| {
            let start = caps[1].parse::<usize>().unwrap();
            let end = caps[2].parse::<usize>().unwrap();
            let truncate = end - start > 40;
            let end = if truncate { start + 40 } else { end };

            format!(
                "\"{}{}\"{}",
                file_content[start..end]
                    .replace('\"', "\\\"")
                    .replace('\t', "\\t")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r"),
                if truncate { "..." } else { "" },
                if truncate { " (truncated)" } else { "" }
            )
        })
        .into_owned()
}

#[derive(new)]
pub struct FusionFileContent {
    pub file_name: PathBuf,
    pub contents: String,
}

impl FusionFileContent {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<FusionFileContent, Error> {
        Ok(FusionFileContent::new(
            path.as_ref().to_path_buf(),
            read_to_string(path.as_ref())
                .map_err(|err| err_generic!("Failed to load file {}: {}", path.as_ref().display(), err))?,
        ))
    }
    pub fn load_stdin() -> Result<FusionFileContent, Error> {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|err| err_generic!("Failed to load stdin: {}", err))?;
        Ok(FusionFileContent::new(PathBuf::from(""), buf))
    }

    pub fn parse(self, fusion_config: &FusionConfig) -> Result<FusionFile, Error> {
        let ast = parser::parse(&self.file_name, &self.contents, fusion_config)
            .map_err(|error| err_generic!("Failed to parse {:?}: {}", self.file_name, error))?;
        Ok(FusionFile::new(self.file_name, self.contents, ast))
    }
}
