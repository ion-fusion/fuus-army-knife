// Copyright Ion Fusion contributors. All Rights Reserved.
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
        match *self {
            ClobExpr::Newlines(_) => true,
            _ => false,
        }
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

    pub fn item_iter<'a>(&'a self) -> impl Iterator<Item = &'a Expr> {
        self.items
            .iter()
            .filter(|expr| (*expr).is_not_comment_or_newlines())
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
    SExpr(ListData),
    Struct(ListData),
    StructKey(NonAnnotatedStringData),
}
impl Expr {
    pub fn is_newlines(&self) -> bool {
        match *self {
            Expr::Newlines(_) => true,
            _ => false,
        }
    }

    pub fn is_comment(&self) -> bool {
        match *self {
            Expr::CommentBlock(_) => true,
            Expr::CommentLine(_) => true,
            _ => false,
        }
    }

    pub fn is_comment_line(&self) -> bool {
        match *self {
            Expr::CommentLine(_) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            Expr::List(_) => true,
            _ => false,
        }
    }

    pub fn is_struct(&self) -> bool {
        match *self {
            Expr::Struct(_) => true,
            _ => false,
        }
    }

    pub fn is_struct_key(&self) -> bool {
        match self {
            Expr::StructKey(_) => true,
            _ => false,
        }
    }

    pub fn is_sexpr(&self) -> bool {
        match self {
            Expr::SExpr(_) => true,
            _ => false,
        }
    }

    pub fn list_data<'a>(&'a self) -> &'a ListData {
        match self {
            Expr::List(data) => data,
            Expr::SExpr(data) => data,
            Expr::Struct(data) => data,
            _ => panic!("called list_data on a non-list"),
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
            Expr::Atomic(ref atomic) => match atomic.typ {
                AtomicType::Symbol => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn stripped_symbol_value<'a>(&'a self) -> Option<&'a str> {
        self.symbol_value().map(|val| {
            if val.starts_with("'") && val.ends_with("'") {
                &val[1..(val.len() - 1)]
            } else {
                val
            }
        })
    }

    pub fn symbol_value<'a>(&'a self) -> Option<&'a String> {
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
            Expr::Atomic(ref mut data) => data.annotations = annotations,
            Expr::Clob(ref mut data) => data.annotations = annotations,
            Expr::List(ref mut data) => data.annotations = annotations,
            Expr::MultilineString(ref mut data) => data.annotations = annotations,
            Expr::SExpr(ref mut data) => data.annotations = annotations,
            Expr::Struct(ref mut data) => data.annotations = annotations,
            _ => unreachable!(),
        }
        self
    }

    pub fn string_value<'a>(&'a self) -> Option<&'a String> {
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
            Expr::Clob(ref data) => data.count_newlines(),
            Expr::CommentBlock(ref data) => data.value.len(),
            Expr::CommentLine(_) => 1,
            Expr::List(ref data) => data.count_newlines(),
            Expr::MultilineString(ref data) => data.count_newlines(),
            Expr::Newlines(ref data) => data.newline_count as usize,
            Expr::SExpr(ref data) => data.count_newlines(),
            Expr::Struct(ref data) => data.count_newlines(),
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
