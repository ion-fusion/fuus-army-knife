// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast::Expr;
use crate::config::FusionConfig;
use crate::error::Error;
use crate::ist::IntermediateSyntaxTree;
use crate::parser;
use regex::{Captures, Regex};
use std::fs::read_to_string;
use std::path::{Path, PathBuf};

#[derive(new)]
pub struct FusionFile {
    pub file_name: PathBuf,
    pub contents: String,
    pub ast: Vec<Expr>,
    pub ist: IntermediateSyntaxTree,
}

impl FusionFile {
    pub fn debug_ast(&self) -> String {
        let debug_view = format!("{:#?}", self.ast);
        replace_spans(&self.contents, &debug_view)
    }

    pub fn debug_ist(&self) -> String {
        let debug_view = format!("{:#?}", self.ist.expressions);
        replace_spans(&self.contents, &debug_view)
    }

    #[cfg(test)]
    pub fn test_file_with_ast(contents: &str, ast: Vec<Expr>) -> FusionFile {
        FusionFile {
            file_name: "test".into(),
            contents: contents.into(),
            ast,
            ist: IntermediateSyntaxTree::new(Vec::new()),
        }
    }
}

fn replace_spans(file_content: &str, debug_view: &str) -> String {
    let span_finder = Regex::new(r"\[Span\((\d+)->(\d+)\)\]").unwrap();
    span_finder
        .replace_all(&debug_view, |caps: &Captures<'_>| {
            let start = caps[1].parse::<usize>().unwrap();
            let end = caps[2].parse::<usize>().unwrap();
            let truncate = end - start > 40;
            let end = if truncate { start + 40 } else { end };

            format!(
                "\"{}{}\"{}",
                (&file_content[start..end])
                    .replace("\"", "\\\"")
                    .replace("\t", "\\t")
                    .replace("\n", "\\n")
                    .replace("\r", "\\r"),
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
            read_to_string(path.as_ref()).map_err(|err| {
                Error::Generic(format!(
                    "Failed to load file {}: {}",
                    path.as_ref().display(),
                    err
                ))
            })?,
        ))
    }

    pub fn parse(self, fusion_config: &FusionConfig) -> Result<FusionFile, Error> {
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

        Ok(FusionFile::new(self.file_name, self.contents, ast, ist))
    }
}
