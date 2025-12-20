// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

/// Macro that produces `Error::Generic` using `format!(...)` syntax
macro_rules! err_generic {
    ($($arg:expr),*) => {
      Error::Generic(format!($($arg,)*))
    }
}

/// Macro that produces `Error::Spanned` using `format!(...)` syntax
macro_rules! err_spanned {
    ($span:expr, $($arg:expr),*) => {
      Error::Spanned($span, format!($($arg,)*))
    }
}
