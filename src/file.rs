// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::Expr;
use crate::config::FusionConfig;
use crate::error::Error;
use crate::parser;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

pub struct FusionFile<'i> {
    pub file_name: &'i Path,
    pub ast: Vec<Expr<'i>>,
}

pub struct FusionFileContent {
    pub file_name: PathBuf,
    pub contents: String,
}

impl FusionFileContent {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<FusionFileContent, Error> {
        Ok(FusionFileContent {
            file_name: path.as_ref().to_path_buf(),
            contents: read_to_string(path.as_ref()).map_err(|err| {
                Error::Generic(format!(
                    "Failed to load file {}: {}",
                    path.as_ref().display(),
                    err
                ))
            })?,
        })
    }

    pub fn parse<'i>(&'i self, fusion_config: &FusionConfig) -> Result<FusionFile<'i>, Error> {
        parser::parse(&self.file_name, &self.contents, &fusion_config)
            .map_err(|error| {
                Error::Generic(format!("Failed to parse {:?}: {}", self.file_name, error))
            })
            .map(|parse_result| FusionFile {
                file_name: &self.file_name,
                ast: parse_result,
            })
    }
}
