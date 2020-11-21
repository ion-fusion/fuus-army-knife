// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::span::ShortSpan;

#[derive(new, Debug)]
pub struct NonAnnotatedValue {
    pub span: ShortSpan,
    pub value: String,
}

#[derive(Debug)]
pub struct NonAnnotatedValues {
    pub span: ShortSpan,
    pub values: Vec<String>,
}
impl NonAnnotatedValues {
    pub fn new(span: ShortSpan, values: Vec<String>) -> NonAnnotatedValues {
        NonAnnotatedValues { span, values }
    }
}

#[derive(Debug)]
pub struct ValueNode {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: String,
}
impl ValueNode {
    pub fn new(span: ShortSpan, value: String) -> ValueNode {
        ValueNode {
            span,
            annotations: Vec::new(),
            value,
        }
    }
}

#[derive(Debug)]
pub struct ValuesNode {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: Vec<String>,
}

#[derive(Debug)]
pub struct ExpressionsNode {
    pub span: ShortSpan,
    pub annotations: Vec<String>,
    pub value: Vec<Expr>,
}
impl ExpressionsNode {
    pub fn new(span: ShortSpan, value: Vec<Expr>) -> ExpressionsNode {
        ExpressionsNode {
            span,
            annotations: Vec::new(),
            value,
        }
    }
}

#[derive(new, Debug)]
pub struct NewlinesNode {
    pub span: ShortSpan,
    pub newlines: u16,
}

#[derive(new, Debug)]
pub struct StructMemberNode {
    pub span: ShortSpan,
    // Includes the key, comments, newlines, and the member itself
    pub value: Vec<Expr>,
}

#[derive(Debug)]
pub enum Expr {
    Blob(ValueNode),
    Boolean(ValueNode),
    Clob(ExpressionsNode),
    CommentBlock(NonAnnotatedValues),
    CommentLine(NonAnnotatedValue),
    Integer(ValueNode),
    List(ExpressionsNode),
    MultilineString(ValueNode),
    Newlines(NewlinesNode),
    Null(ValueNode),
    QuotedString(ValueNode),
    Real(ValueNode),
    SExpr(ExpressionsNode),
    Struct(ExpressionsNode),
    StructKey(ValueNode),
    StructMember(StructMemberNode),
    Symbol(ValueNode),
    Timestamp(ValueNode),
}
impl Expr {
    pub fn attach_annotations(mut self: Expr, annotations: Vec<String>) -> Expr {
        match &mut self {
            Expr::Blob(ref mut value) => value.annotations = annotations,
            Expr::Boolean(ref mut value) => value.annotations = annotations,
            Expr::Clob(ref mut value) => value.annotations = annotations,
            Expr::Integer(ref mut value) => value.annotations = annotations,
            Expr::List(ref mut value) => value.annotations = annotations,
            Expr::MultilineString(ref mut value) => value.annotations = annotations,
            Expr::Null(ref mut value) => value.annotations = annotations,
            Expr::QuotedString(ref mut value) => value.annotations = annotations,
            Expr::Real(ref mut value) => value.annotations = annotations,
            Expr::SExpr(ref mut value) => value.annotations = annotations,
            Expr::Struct(ref mut value) => value.annotations = annotations,
            Expr::Symbol(ref mut value) => value.annotations = annotations,
            Expr::Timestamp(ref mut value) => value.annotations = annotations,
            _ => unreachable!(),
        }
        self
    }

    pub fn span(&self) -> ShortSpan {
        match *self {
            Expr::Blob(ref value) => value.span,
            Expr::Boolean(ref value) => value.span,
            Expr::Clob(ref value) => value.span,
            Expr::CommentBlock(ref value) => value.span,
            Expr::CommentLine(ref value) => value.span,
            Expr::Integer(ref value) => value.span,
            Expr::List(ref value) => value.span,
            Expr::MultilineString(ref value) => value.span,
            Expr::Newlines(ref value) => value.span,
            Expr::Null(ref value) => value.span,
            Expr::QuotedString(ref value) => value.span,
            Expr::Real(ref value) => value.span,
            Expr::SExpr(ref value) => value.span,
            Expr::Struct(ref value) => value.span,
            Expr::StructKey(ref value) => value.span,
            Expr::StructMember(ref value) => value.span,
            Expr::Symbol(ref value) => value.span,
            Expr::Timestamp(ref value) => value.span,
        }
    }
}
