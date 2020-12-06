// Copyright Ion Fusion contributors. All Rights Reserved.

/// Macro that produces Error::Generic using `format!(...)` syntax
macro_rules! err_generic {
    ($($arg:expr),*) => {
      crate::error::Error::Generic(format!($($arg,)*))
    }
}

/// Macro that produces Error::Spanned using `format!(...)` syntax
macro_rules! err_spanned {
    ($span:expr, $($arg:expr),*) => {
      crate::error::Error::Spanned($span, format!($($arg,)*))
    }
}

/// Macro that prints to stderr using `format!(...)` syntax and then exits with status 1
macro_rules! bail {
    ($($arg:expr),*) => {
        {
            eprintln!($($arg,)*);
            ::std::process::exit(1);
        }
    };
}
