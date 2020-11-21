// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::Expr;
use crate::config::FusionConfig;
use crate::error::Error;
use crate::ist::IntermediateSyntaxTree;
use crate::parser;
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

#[derive(new)]
pub struct FusionFile<'i> {
    pub file_name: &'i Path,
    pub ast: Vec<Expr<'i>>,
    pub ist: IntermediateSyntaxTree,
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
            read_to_string(path.as_ref()).map_err(|err| {
                Error::Generic(format!(
                    "Failed to load file {}: {}",
                    path.as_ref().display(),
                    err
                ))
            })?,
        ))
    }

    pub fn parse<'i>(&'i self, fusion_config: &FusionConfig) -> Result<FusionFile<'i>, Error> {
        let ast =
            parser::parse(&self.file_name, &self.contents, &fusion_config).map_err(|error| {
                Error::Generic(format!("Failed to parse {:?}: {}", self.file_name, error))
            })?;
        let ist = IntermediateSyntaxTree::from_ast(&ast).map_err(|error| {
            Error::Generic(format!(
                "Failed to translate AST to IST for {:?}: {}",
                self.file_name, error
            ))
        })?;

        Ok(FusionFile::new(&self.file_name, ast, ist))
    }
}
