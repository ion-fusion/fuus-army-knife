// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::lexer::Rule;
use std::fmt;
use std::fmt::Display;

#[derive(Clone, Debug)]
pub enum Error {
    Generic(String),
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Generic(ref message) => message,
        }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Generic(ref message) => formatter.write_str(message),
        }
    }
}

impl From<pest::error::Error<Rule>> for Error {
    fn from(error: pest::error::Error<Rule>) -> Self {
        Error::Generic(error.to_string())
    }
}
