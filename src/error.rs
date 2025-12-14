// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::lexer::Rule;
use crate::span::ShortSpan;
use std::fmt;
use std::fmt::Display;
use std::path::Path;

#[derive(Clone, Debug)]
pub enum Error {
    /// Generic error message
    Generic(String),
    /// Error message with a span that needs to be converted
    /// into a generic error to be human-friendly.
    Spanned(ShortSpan, String),
}

impl Error {
    /// Converts a spanned error into a generic error
    pub fn resolve_spanned<P: AsRef<Path>>(self, file_name: P, file_contents: &str) -> Error {
        use pest::Span;
        use pest::error::{Error as PestError, ErrorVariant};
        match self {
            Error::Generic(msg) => Error::Generic(msg),
            Error::Spanned(span, msg) => {
                let pest_span = Span::new(file_contents, span.start, span.end).unwrap();
                let pest_error = PestError::new_from_span(
                    ErrorVariant::<crate::lexer::Rule>::CustomError { message: msg },
                    pest_span,
                )
                .with_path(&file_name.as_ref().as_os_str().to_string_lossy());
                err_generic!("{}", pest_error.to_string())
            }
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Generic(ref message) => message,
            Error::Spanned(_span, ref message) => message,
        }
    }
}

impl Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Error::Generic(ref message) => formatter.write_str(message),
            Error::Spanned(_span, ref message) => formatter.write_str(message),
        }
    }
}

impl From<pest::error::Error<Rule>> for Error {
    fn from(error: pest::error::Error<Rule>) -> Self {
        Error::Generic(error.to_string())
    }
}
