// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::span::ShortSpan;
use crate::string_util::count_newlines;
use std::fmt;

pub trait CountNewlines {
    fn count_newlines(&self) -> usize;
}

pub trait CountItemsBeforeNewline {
    fn count_items_before_newline(&self) -> usize;
}

pub trait CountUntilPred<P, U>
where
    P: Fn(&Expr) -> bool,
    U: Fn(&Expr) -> bool,
{
    fn count_until(&self, pred: P, until: U) -> usize;
}

#[derive(new, Clone, Debug)]
pub struct NonAnnotatedStringData {
    pub span: ShortSpan,
    pub value: String,
}

#[derive(new, Clone, Debug)]
pub struct NonAnnotatedStringListData {
    pub span: ShortSpan,
    pub value: Vec<String>,
}

#[derive(new, Clone, Debug)]
pub struct MultilineStringData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: String,
}

impl CountNewlines for MultilineStringData {
    fn count_newlines(&self) -> usize {
        count_newlines(&self.value)
    }
}

#[derive(Clone, Debug)]
pub enum ClobExpr {
    MultilineString(MultilineStringData),
    QuotedString(AtomicData),
    Newlines(NewlinesData),
}

impl ClobExpr {
    pub fn is_newlines(&self) -> bool {
        matches!(*self, ClobExpr::Newlines(_))
    }
}

impl CountItemsBeforeNewline for &[ClobExpr] {
    fn count_items_before_newline(&self) -> usize {
        let mut count = 0;
        for expr in *self {
            match expr {
                ClobExpr::Newlines(_) => return count,
                _ => {
                    count += 1;
                }
            }
        }
        count
    }
}

#[derive(new, Clone, Debug)]
pub struct ClobData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub clobs: Vec<ClobExpr>,
}
impl CountNewlines for ClobData {
    fn count_newlines(&self) -> usize {
        let mut total = 0;
        for expr in &self.clobs {
            total += match *expr {
                ClobExpr::MultilineString(ref data) => data.value.len(),
                ClobExpr::QuotedString(_) => 0,
                ClobExpr::Newlines(ref data) => data.newline_count as usize,
            }
        }
        total
    }
}

#[derive(new, Clone, Debug)]
pub struct ListData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub items: Vec<Expr>,
}
impl ListData {
    pub fn count_newlines(&self) -> usize {
        self.items.iter().map(|expr| expr.count_newlines()).sum()
    }

    pub fn item_iter(&self) -> impl Iterator<Item = &'_ Expr> {
        self.items.iter().filter(|expr| (*expr).is_not_comment_or_newlines())
    }
}

#[derive(new, Clone, Copy)]
pub struct NewlinesData {
    pub span: ShortSpan,
    pub newline_count: u16,
}
impl fmt::Debug for NewlinesData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "NewlinesData {{ {:#?}, newline_count: {} }}",
            self.span, self.newline_count
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum AtomicType {
    Blob,
    Boolean,
    Integer,
    Null,
    QuotedString,
    Real,
    Symbol,
    Timestamp,
}

#[derive(new, Clone, Debug)]
pub struct AtomicData {
    pub typ: AtomicType,
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: String,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Atomic(AtomicData),
    Clob(ClobData),
    CommentBlock(NonAnnotatedStringListData),
    CommentLine(NonAnnotatedStringData),
    List(ListData),
    MultilineString(MultilineStringData),
    Newlines(NewlinesData),
    #[allow(clippy::enum_variant_names)]
    SExpr(ListData),
    Struct(ListData),
    StructKey(NonAnnotatedStringData),
}
#[allow(dead_code)]
impl Expr {
    pub fn is_newlines(&self) -> bool {
        matches!(*self, Expr::Newlines(_))
    }

    pub fn is_comment(&self) -> bool {
        matches!(*self, Expr::CommentBlock(_) | Expr::CommentLine(_))
    }

    pub fn is_comment_line(&self) -> bool {
        matches!(*self, Expr::CommentLine(_))
    }

    pub fn _is_list(&self) -> bool {
        matches!(self, Expr::List(_))
    }

    pub fn is_struct(&self) -> bool {
        matches!(*self, Expr::Struct(_))
    }

    pub fn is_struct_key(&self) -> bool {
        matches!(self, Expr::StructKey(_))
    }

    pub fn is_sexpr(&self) -> bool {
        matches!(self, Expr::SExpr(_))
    }

    pub fn into_struct_value(self) -> Option<ListData> {
        match self {
            Expr::Struct(data) => Some(data),
            _ => None,
        }
    }

    pub fn sexpr_value(&self) -> Option<&ListData> {
        match self {
            Expr::SExpr(data) => Some(data),
            _ => None,
        }
    }

    pub fn struct_value(&self) -> Option<&ListData> {
        match self {
            Expr::Struct(data) => Some(data),
            _ => None,
        }
    }

    pub fn list_value(&self) -> Option<&ListData> {
        match self {
            Expr::List(data) => Some(data),
            _ => None,
        }
    }

    pub fn is_not_comment_or_newlines(&self) -> bool {
        !self.is_newlines() && !self.is_comment()
    }

    pub fn is_value(&self) -> bool {
        !self.is_newlines() && !self.is_comment() && !self.is_struct_key()
    }

    pub fn is_symbol(&self) -> bool {
        match *self {
            Expr::Atomic(ref atomic) => matches!(atomic.typ, AtomicType::Symbol),
            _ => false,
        }
    }

    pub fn stripped_symbol_value(&self) -> Option<&str> {
        self.symbol_value().map(|val| {
            if val.starts_with('\'') && val.ends_with('\'') {
                &val[1..(val.len() - 1)]
            } else {
                val
            }
        })
    }

    pub fn symbol_value(&self) -> Option<&String> {
        match *self {
            Expr::Atomic(ref atomic) => match atomic.typ {
                AtomicType::Symbol => Some(&atomic.value),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn span(&self) -> ShortSpan {
        match self {
            Expr::Atomic(data) => data.span,
            Expr::Clob(data) => data.span,
            Expr::CommentBlock(data) => data.span,
            Expr::CommentLine(data) => data.span,
            Expr::List(data) => data.span,
            Expr::MultilineString(data) => data.span,
            Expr::Newlines(data) => data.span,
            Expr::SExpr(data) => data.span,
            Expr::Struct(data) => data.span,
            Expr::StructKey(data) => data.span,
        }
    }

    pub fn attach_annotations(mut self: Expr, annotations: Vec<String>) -> Expr {
        match &mut self {
            Expr::Atomic(data) => data.annotations = annotations,
            Expr::Clob(data) => data.annotations = annotations,
            Expr::List(data) => data.annotations = annotations,
            Expr::MultilineString(data) => data.annotations = annotations,
            Expr::SExpr(data) => data.annotations = annotations,
            Expr::Struct(data) => data.annotations = annotations,
            _ => unreachable!(),
        }
        self
    }

    pub fn string_value(&self) -> Option<&String> {
        match self {
            Expr::Atomic(data) => match data.typ {
                AtomicType::QuotedString => Some(&data.value),
                _ => None,
            },
            Expr::MultilineString(data) => Some(&data.value),
            _ => None,
        }
    }
}
impl CountNewlines for &Expr {
    fn count_newlines(&self) -> usize {
        match *self {
            Expr::Atomic(_) => 0,
            Expr::Clob(data) => data.count_newlines(),
            Expr::CommentBlock(data) => data.value.len(),
            Expr::CommentLine(_) => 1,
            Expr::List(data) => data.count_newlines(),
            Expr::MultilineString(data) => data.count_newlines(),
            Expr::Newlines(data) => data.newline_count as usize,
            Expr::SExpr(data) => data.count_newlines(),
            Expr::Struct(data) => data.count_newlines(),
            Expr::StructKey(_) => 0,
        }
    }
}

impl CountNewlines for &[Expr] {
    fn count_newlines(&self) -> usize {
        self.iter().map(|expr| expr.count_newlines()).sum()
    }
}
impl<P, U> CountUntilPred<P, U> for &[Expr]
where
    P: Fn(&Expr) -> bool,
    U: Fn(&Expr) -> bool,
{
    fn count_until(&self, pred: P, until: U) -> usize {
        let mut count = 0;
        for expr in *self {
            if pred(expr) {
                count += 1;
            } else if until(expr) {
                return count;
            }
        }
        count
    }
}

impl CountItemsBeforeNewline for &[Expr] {
    fn count_items_before_newline(&self) -> usize {
        self.count_until(|e| e.is_not_comment_or_newlines(), |e| e.is_newlines())
    }
}
