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

#[derive(new, Debug)]
pub struct NonAnnotatedStringData {
    pub span: ShortSpan,
    pub value: String,
}

#[derive(new, Debug)]
pub struct NonAnnotatedStringListData {
    pub span: ShortSpan,
    pub value: Vec<String>,
}

#[derive(new, Debug)]
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

#[derive(Debug)]
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

#[derive(new, Debug)]
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

#[derive(new, Debug)]
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

#[derive(new, Debug)]
pub struct StructMemberData {
    pub span: ShortSpan,
    pub items: Vec<IExpr>,
}

#[derive(new, Debug)]
pub enum StructExpr {
    StructKey(NonAnnotatedStringData),
    StructValue(IExpr),
}
impl StructExpr {
    pub fn is_key(&self) -> bool {
        match self {
            StructExpr::StructKey(_) => true,
            _ => false,
        }
    }

    pub fn is_nested_struct(&self) -> bool {
        match self {
            StructExpr::StructValue(expr) => expr.is_struct(),
            _ => false,
        }
    }

    pub fn is_value(&self) -> bool {
        match self {
            StructExpr::StructValue(expr) => expr.is_value(),
            _ => false,
        }
    }
}

#[derive(new, Debug)]
pub struct StructData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub items: Vec<StructExpr>,
}
impl CountNewlines for StructData {
    fn count_newlines(&self) -> usize {
        let mut total = 0;
        for expr in &self.items {
            total += match *expr {
                StructExpr::StructValue(ref data) => data.count_newlines(),
                _ => 0,
            }
        }
        total
    }
}

#[derive(new)]
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

#[derive(new, Debug)]
pub struct AtomicData {
    pub typ: AtomicType,
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: String,
}

#[derive(Debug)]
pub enum IExpr {
    Atomic(AtomicData),
    Clob(ClobData),
    CommentBlock(NonAnnotatedStringListData),
    CommentLine(NonAnnotatedStringData),
    List(ListData),
    MultilineString(MultilineStringData),
    Newlines(NewlinesData),
    SExpr(ListData),
    Struct(StructData),
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

    pub fn is_struct(&self) -> bool {
        match *self {
            IExpr::Struct(_) => true,
            _ => false,
        }
    }

    pub fn is_value(&self) -> bool {
        !self.is_newlines() && !self.is_comment()
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
        }
    }
}

impl CountNewlines for &[IExpr] {
    fn count_newlines(&self) -> usize {
        self.iter().map(|expr| expr.count_newlines()).sum()
    }
}

impl CountItemsBeforeNewline for &[IExpr] {
    fn count_items_before_newline(&self) -> usize {
        let mut count = 0;
        for expr in *self {
            if expr.is_value() {
                count += 1;
            } else if expr.is_newlines() {
                return count;
            }
        }
        count
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

fn visit_ast_struct_member(exprs: &Vec<ast::Expr>) -> Result<Vec<StructExpr>, Error> {
    let mut ist: Vec<StructExpr> = Vec::new();
    for ast_mem in exprs {
        ist.push(match ast_mem {
            ast::Expr::StructKey(ref key) => {
                StructExpr::StructKey(NonAnnotatedStringData::new(key.span, key.value.clone()))
            }
            _ => StructExpr::StructValue(visit_ast_expr(ast_mem)?),
        })
    }
    Ok(ist)
}

fn visit_ast_struct(expr: &ast::ExpressionsNode, span: ShortSpan) -> Result<IExpr, Error> {
    let mut ist: Vec<StructExpr> = Vec::new();
    for ast_mem in &expr.value {
        match *ast_mem {
            ast::Expr::StructMember(ref mem) => {
                ist.extend(visit_ast_struct_member(&mem.value)?.into_iter())
            }
            ast::Expr::CommentBlock(_) | ast::Expr::CommentLine(_) | ast::Expr::Newlines(_) => {
                ist.push(StructExpr::StructValue(visit_ast_expr(ast_mem)?))
            }
            _ => unreachable!(),
        }
    }
    Ok(IExpr::Struct(StructData::new(
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
