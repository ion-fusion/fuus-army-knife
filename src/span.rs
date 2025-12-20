// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use derive_new::new;
use pest::Span;
use std::fmt;

// Copyable alternative to Pest's Span
#[derive(new, Clone, Copy, PartialEq, Eq)]
pub struct ShortSpan {
    pub start: usize,
    pub end: usize,
}
impl From<Span<'_>> for ShortSpan {
    fn from(other: Span<'_>) -> ShortSpan {
        ShortSpan {
            start: other.start(),
            end: other.end(),
        }
    }
}
impl fmt::Debug for ShortSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[Span({}->{})]", self.start, self.end)
    }
}
