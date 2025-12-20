// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

/// Macro that prints to stderr using `format!(...)` syntax and then exits with status 1
macro_rules! bail {
    ($($arg:expr),*) => {
        {
            eprintln!($($arg,)*);
            ::std::process::exit(1);
        }
    };
}
