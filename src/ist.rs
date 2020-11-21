// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ast;
use crate::error::Error;
use pest::Span;
use std::fmt;

#[derive(new, Clone, Copy, PartialEq, Eq)]
pub struct ShortSpan {
    pub start: usize,
    pub end: usize,
}
impl From<&Span<'_>> for ShortSpan {
    fn from(other: &Span<'_>) -> ShortSpan {
        ShortSpan {
            start: other.start(),
            end: other.end(),
        }
    }
}
impl fmt::Debug for ShortSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Span({}, {})", self.start, self.end)
    }
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
pub struct AnnotatedStringListData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: Vec<String>,
}

#[derive(Debug)]
pub enum ClobExpr {
    MultilineString(AnnotatedStringListData),
    QuotedString(AtomicData),
    Newlines(NewlinesData),
}

#[derive(new, Debug)]
pub struct ClobData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub clobs: Vec<ClobExpr>,
}

#[derive(new, Debug)]
pub struct ListData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub items: Vec<IExpr>,
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

#[derive(new, Debug)]
pub struct StructData {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub items: Vec<StructExpr>,
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
    MultilineString(AnnotatedStringListData),
    Newlines(NewlinesData),
    SExpr(ListData),
    Struct(StructData),
}

#[derive(new)]
pub struct IntermediateSyntaxTree {
    pub expressions: Vec<IExpr>,
}

impl<'i> IntermediateSyntaxTree {
    pub fn from_ast(from: &Vec<ast::Expr<'_>>) -> Result<IntermediateSyntaxTree, Error> {
        Ok(IntermediateSyntaxTree::new(visit_ast_exprs(from)?))
    }
}

fn visit_ast_exprs(exprs: &Vec<ast::Expr<'_>>) -> Result<Vec<IExpr>, Error> {
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

fn visit_ast_expr(expr: &ast::Expr<'_>) -> Result<IExpr, Error> {
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

fn visit_ast_struct_member(exprs: &Vec<ast::Expr<'_>>) -> Result<Vec<StructExpr>, Error> {
    let mut ist: Vec<StructExpr> = Vec::new();
    for ast_mem in exprs {
        ist.push(match ast_mem {
            ast::Expr::StructKey(ref key) => StructExpr::StructKey(NonAnnotatedStringData::new(
                (&key.span).into(),
                key.value.clone(),
            )),
            _ => StructExpr::StructValue(visit_ast_expr(ast_mem)?),
        })
    }
    Ok(ist)
}

fn visit_ast_struct(expr: &ast::ExpressionsNode<'_>, span: ShortSpan) -> Result<IExpr, Error> {
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
    expr: &ast::NonAnnotatedValues<'_>,
    span: ShortSpan,
) -> Result<IExpr, Error> {
    Ok(IExpr::CommentBlock(NonAnnotatedStringListData::new(
        span,
        expr.values.clone(),
    )))
}

fn visit_ast_line_comment(
    expr: &ast::NonAnnotatedValue<'_>,
    span: ShortSpan,
) -> Result<IExpr, Error> {
    Ok(IExpr::CommentLine(NonAnnotatedStringData::new(
        span,
        expr.value.clone(),
    )))
}

fn visit_ast_clob(expr: &ast::ExpressionsNode<'_>, span: ShortSpan) -> Result<IExpr, Error> {
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

fn visit_ast_multiline_string(expr: &ast::ValuesNode<'_>, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::MultilineString(AnnotatedStringListData::new(
        span,
        expr.annotations.clone(),
        expr.value.clone(),
    )))
}

fn visit_ast_list(expr: &ast::ExpressionsNode<'_>, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::List(ListData::new(
        span,
        expr.annotations.clone(),
        visit_ast_exprs(&expr.value)?,
    )))
}

fn visit_ast_sexpr(expr: &ast::ExpressionsNode<'_>, span: ShortSpan) -> Result<IExpr, Error> {
    Ok(IExpr::SExpr(ListData::new(
        span,
        expr.annotations.clone(),
        visit_ast_exprs(&expr.value)?,
    )))
}
