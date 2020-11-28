// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast;
use crate::error::Error;
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
    P: Fn(&IExpr) -> bool,
    U: Fn(&IExpr) -> bool,
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
    pub items: Vec<IExpr>,
}
impl ListData {
    pub fn count_newlines(&self) -> usize {
        self.items.iter().map(|expr| expr.count_newlines()).sum()
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

#[derive(Debug, Copy, Clone)]
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
pub enum IExpr {
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
impl IExpr {
    pub fn is_newlines(&self) -> bool {
        match *self {
            IExpr::Newlines(_) => true,
            _ => false,
        }
    }

    pub fn is_comment(&self) -> bool {
        match *self {
            IExpr::CommentBlock(_) => true,
            IExpr::CommentLine(_) => true,
            _ => false,
        }
    }

    pub fn is_comment_line(&self) -> bool {
        match *self {
            IExpr::CommentLine(_) => true,
            _ => false,
        }
    }

    pub fn is_list(&self) -> bool {
        match self {
            IExpr::List(_) => true,
            _ => false,
        }
    }

    pub fn is_struct(&self) -> bool {
        match *self {
            IExpr::Struct(_) => true,
            _ => false,
        }
    }

    pub fn is_struct_key(&self) -> bool {
        match self {
            IExpr::StructKey(_) => true,
            _ => false,
        }
    }

    pub fn is_sexpr(&self) -> bool {
        match self {
            IExpr::SExpr(_) => true,
            _ => false,
        }
    }

    pub fn list_data<'a>(&'a self) -> &'a ListData {
        match self {
            IExpr::List(data) => data,
            IExpr::SExpr(data) => data,
            IExpr::Struct(data) => data,
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
            IExpr::Atomic(ref atomic) => match atomic.typ {
                AtomicType::Symbol => true,
                _ => false,
            },
            _ => false,
        }
    }

    pub fn symbol_value<'a>(&'a self) -> &'a String {
        match *self {
            IExpr::Atomic(ref atomic) => match atomic.typ {
                AtomicType::Symbol => &atomic.value,
                _ => panic!(),
            },
            _ => panic!(),
        }
    }

    pub fn span(&self) -> ShortSpan {
        match self {
            IExpr::Atomic(data) => data.span,
            IExpr::Clob(data) => data.span,
            IExpr::CommentBlock(data) => data.span,
            IExpr::CommentLine(data) => data.span,
            IExpr::List(data) => data.span,
            IExpr::MultilineString(data) => data.span,
            IExpr::Newlines(data) => data.span,
            IExpr::SExpr(data) => data.span,
            IExpr::Struct(data) => data.span,
            IExpr::StructKey(data) => data.span,
        }
    }
}
impl CountNewlines for &IExpr {
    fn count_newlines(&self) -> usize {
        match *self {
            IExpr::Atomic(_) => 0,
            IExpr::Clob(ref data) => data.count_newlines(),
            IExpr::CommentBlock(ref data) => data.value.len(),
            IExpr::CommentLine(_) => 1,
            IExpr::List(ref data) => data.count_newlines(),
            IExpr::MultilineString(ref data) => data.count_newlines(),
            IExpr::Newlines(ref data) => data.newline_count as usize,
            IExpr::SExpr(ref data) => data.count_newlines(),
            IExpr::Struct(ref data) => data.count_newlines(),
            IExpr::StructKey(_) => 0,
        }
    }
}

impl CountNewlines for &[IExpr] {
    fn count_newlines(&self) -> usize {
        self.iter().map(|expr| expr.count_newlines()).sum()
    }
}
impl<P, U> CountUntilPred<P, U> for &[IExpr]
where
    P: Fn(&IExpr) -> bool,
    U: Fn(&IExpr) -> bool,
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

impl CountItemsBeforeNewline for &[IExpr] {
    fn count_items_before_newline(&self) -> usize {
        self.count_until(|e| e.is_not_comment_or_newlines(), |e| e.is_newlines())
    }
}

#[derive(new)]
pub struct IntermediateSyntaxTree {
    pub expressions: Vec<IExpr>,
}

impl IntermediateSyntaxTree {
    pub fn from_ast(from: &Vec<ast::Expr>) -> Result<IntermediateSyntaxTree, Error> {
        Ok(IntermediateSyntaxTree::new(visit_ast_exprs(from)?))
    }
}

fn visit_ast_exprs(exprs: &Vec<ast::Expr>) -> Result<Vec<IExpr>, Error> {
    exprs
        .iter()
        .map(|expr| visit_ast_expr(expr))
        .try_fold(Vec::new(), |mut v, iexpr| {
            v.push(iexpr?);
            Ok(v)
        })
}

macro_rules! atomic_value {
    ($typ:ident,$span:ident, $value:ident) => {
        Ok(IExpr::Atomic(AtomicData::new(
            AtomicType::$typ,
            $span,
            $value.annotations.clone(),
            $value.value.clone(),
        )))
    };
}

fn visit_ast_expr(expr: &ast::Expr) -> Result<IExpr, Error> {
    let span: ShortSpan = expr.span().into();
    match *expr {
        ast::Expr::Blob(ref value) => atomic_value!(Blob, span, value),
        ast::Expr::Boolean(ref value) => atomic_value!(Boolean, span, value),
        ast::Expr::Clob(ref value) => visit_ast_clob(value, span),
        ast::Expr::CommentBlock(ref value) => visit_ast_block_comment(value, span),
        ast::Expr::CommentLine(ref value) => visit_ast_line_comment(value, span),
        ast::Expr::Integer(ref value) => atomic_value!(Integer, span, value),
        ast::Expr::List(ref value) => visit_ast_list(value, span),
        ast::Expr::MultilineString(ref value) => visit_ast_multiline_string(value, span),
        ast::Expr::Newlines(ref value) => {
            Ok(IExpr::Newlines(NewlinesData::new(span, value.newlines)))
        }
        ast::Expr::Null(ref value) => atomic_value!(Null, span, value),
        ast::Expr::QuotedString(ref value) => atomic_value!(QuotedString, span, value),
        ast::Expr::Real(ref value) => atomic_value!(Real, span, value),
        ast::Expr::SExpr(ref value) => visit_ast_sexpr(value, span),
        ast::Expr::Struct(ref value) => visit_ast_struct(value, span),
        ast::Expr::Symbol(ref value) => atomic_value!(Symbol, span, value),
        ast::Expr::Timestamp(ref value) => atomic_value!(Timestamp, span, value),

        ast::Expr::StructKey(_) | ast::Expr::StructMember(_) => unreachable!(),
    }
}

fn visit_ast_struct_member(exprs: &Vec<ast::Expr>) -> Result<Vec<IExpr>, Error> {
    let mut ist: Vec<IExpr> = Vec::new();
    for ast_mem in exprs {
        ist.push(match ast_mem {
            ast::Expr::StructKey(ref key) => {
                IExpr::StructKey(NonAnnotatedStringData::new(key.span, key.value.clone()))
            }
            _ => visit_ast_expr(ast_mem)?,
        })
    }
    Ok(ist)
}

fn visit_ast_struct(expr: &ast::ExpressionsNode, span: ShortSpan) -> Result<IExpr, Error> {
    let mut ist: Vec<IExpr> = Vec::new();
    for ast_mem in &expr.value {
        match *ast_mem {
            ast::Expr::StructMember(ref mem) => {
                ist.extend(visit_ast_struct_member(&mem.value)?.into_iter())
            }
            ast::Expr::CommentBlock(_) | ast::Expr::CommentLine(_) | ast::Expr::Newlines(_) => {
                ist.push(visit_ast_expr(ast_mem)?)
            }
            _ => unreachable!(),
        }
    }
    Ok(IExpr::Struct(ListData::new(
        span,
        expr.annotations.clone(),
        ist,
    )))
}

fn visit_ast_block_comment(
    expr: &ast::NonAnnotatedValues,
    span: ShortSpan,
) -> Result<IExpr, Error> {
    Ok(IExpr::CommentBlock(NonAnnotatedStringListData::new(
        span,
        expr.values.clone(),
    )))
}

fn visit_ast_line_comment(expr: &ast::NonAnnotatedValue, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::CommentLine(NonAnnotatedStringData::new(
        span,
        expr.value.clone(),
    )))
}

fn visit_ast_clob(expr: &ast::ExpressionsNode, span: ShortSpan) -> Result<IExpr, Error> {
    let ist: Vec<ClobExpr> = visit_ast_exprs(&expr.value)?
        .into_iter()
        .map(|iexpr| match iexpr {
            IExpr::MultilineString(value) => ClobExpr::MultilineString(value),
            IExpr::Atomic(value) => match value.typ {
                AtomicType::QuotedString => ClobExpr::QuotedString(value),
                _ => unreachable!(),
            },
            IExpr::Newlines(value) => ClobExpr::Newlines(value),
            _ => unreachable!(),
        })
        .collect();
    Ok(IExpr::Clob(ClobData::new(
        span,
        expr.annotations.clone(),
        ist,
    )))
}

fn visit_ast_multiline_string(expr: &ast::ValueNode, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::MultilineString(MultilineStringData::new(
        span,
        expr.annotations.clone(),
        expr.value.clone(),
    )))
}

fn visit_ast_list(expr: &ast::ExpressionsNode, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::List(ListData::new(
        span,
        expr.annotations.clone(),
        visit_ast_exprs(&expr.value)?,
    )))
}

fn visit_ast_sexpr(expr: &ast::ExpressionsNode, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::SExpr(ListData::new(
        span,
        expr.annotations.clone(),
        visit_ast_exprs(&expr.value)?,
    )))
}
