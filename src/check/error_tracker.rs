// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::error::Error;
use crate::span::ShortSpan;
use pest::error::Error as PestError;
use pest::error::ErrorVariant;
use pest::Span;
use std::path::Path;

pub struct ErrorTracker<'i> {
    file_name: String,
    file_contents: &'i str,
    errors: Vec<Error>,
}

impl<'i> ErrorTracker<'i> {
    pub fn new(file_name: &'i Path, file_contents: &'i str) -> ErrorTracker<'i> {
        ErrorTracker {
            file_name: format!("{:?}", file_name),
            file_contents,
            errors: Vec::new(),
        }
    }

    pub fn unbound_ident(&mut self, name: &str, span: ShortSpan) {
        self.custom_error(format!("Unbound identifier {}", name), span);
    }

    pub fn custom_error<S: Into<String>>(&mut self, message: S, span: ShortSpan) {
        let pest_span = Span::new(self.file_contents, span.start, span.end).unwrap();
        let pest_error = PestError::new_from_span(
            ErrorVariant::<crate::lexer::Rule>::CustomError {
                message: message.into(),
            },
            pest_span,
        )
        .with_path(&self.file_name);
        self.errors
            .push(Error::Generic(format!("{}", pest_error.to_string())));
    }

    pub fn into_errors(self) -> Vec<Error> {
        self.errors
    }
}
