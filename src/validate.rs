// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::error::Error;
use crate::file::FusionFile;
use crate::lexer::Rule;
use pest::error::Error as PestError;
use pest::error::ErrorVariant;
use pest::Span;

struct ErrorTracker {
    errors: Vec<Error>,
}

impl ErrorTracker {
    fn new() -> ErrorTracker {
        ErrorTracker { errors: Vec::new() }
    }

    fn unbound_ident(&mut self, name: &str, span: &Span<'_>) {
        self.custom_error(format!("Unbound identifier {}", name), span);
    }

    fn custom_error<S: Into<String>>(&mut self, message: S, span: &Span<'_>) {
        let pest_error = PestError::new_from_span(
            ErrorVariant::<Rule>::CustomError {
                message: message.into(),
            },
            span.clone(),
        );
        self.errors
            .push(Error::Generic(format!("{}", pest_error.to_string())));
    }

    fn into_errors(self) -> Vec<Error> {
        self.errors
    }
}

pub fn validate(file: &FusionFile) -> Vec<Error> {
    let mut tracker = ErrorTracker::new();
    validate_unbound_ident(&mut tracker, file);
    tracker.into_errors()
}

fn validate_unbound_ident(_tracker: &mut ErrorTracker, _file: &FusionFile) {
    // TODO
}
