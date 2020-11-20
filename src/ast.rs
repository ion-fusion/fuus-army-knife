// Copyright Ion Fusion contributors. All Rights Reserved.
use pest::Span;

#[derive(Debug)]
pub struct NonAnnotatedValue<'i> {
    pub span: Span<'i>,
    pub value: String,
}
impl<'i> NonAnnotatedValue<'i> {
    pub fn new(span: Span<'i>, value: String) -> NonAnnotatedValue<'i> {
        NonAnnotatedValue { span, value }
    }
}

#[derive(Debug)]
pub struct NonAnnotatedValues<'i> {
    pub span: Span<'i>,
    pub values: Vec<String>,
}
impl<'i> NonAnnotatedValues<'i> {
    pub fn new(span: Span<'i>, values: Vec<String>) -> NonAnnotatedValues<'i> {
        NonAnnotatedValues { span, values }
    }
}

#[derive(Debug)]
pub struct ValueNode<'i> {
    pub span: Span<'i>,
    pub annotations: Vec<String>,
    pub value: String,
}
impl<'i> ValueNode<'i> {
    pub fn new(span: Span<'i>, value: String) -> ValueNode<'i> {
        ValueNode {
            span,
            annotations: Vec::new(),
            value,
        }
    }
}

#[derive(Debug)]
pub struct ValuesNode<'i> {
    pub span: Span<'i>,
    pub annotations: Vec<String>,
    pub value: Vec<String>,
}
impl<'i> ValuesNode<'i> {
    pub fn new(span: Span<'i>, value: Vec<String>) -> ValuesNode<'i> {
        ValuesNode {
            span,
            annotations: Vec::new(),
            value,
        }
    }
}

#[derive(Debug)]
pub struct ExpressionsNode<'i> {
    pub span: Span<'i>,
    pub annotations: Vec<String>,
    pub value: Vec<Expr<'i>>,
}
impl<'i> ExpressionsNode<'i> {
    pub fn new(span: Span<'i>, value: Vec<Expr<'i>>) -> ExpressionsNode<'i> {
        ExpressionsNode {
            span,
            annotations: Vec::new(),
            value,
        }
    }
}

#[derive(Debug)]
pub struct NewlinesNode<'i> {
    pub span: Span<'i>,
    pub newlines: u16,
}
impl<'i> NewlinesNode<'i> {
    pub fn new(span: Span<'i>, newlines: u16) -> NewlinesNode<'i> {
        NewlinesNode { span, newlines }
    }
}

#[derive(Debug)]
pub struct StructMemberNode<'i> {
    pub span: Span<'i>,
    pub key: Box<Expr<'i>>,
    // It's a Vec since there can be multiple Newlines expressions embedded in there
    pub value: Vec<Expr<'i>>,
}
impl<'i> StructMemberNode<'i> {
    pub fn new(span: Span<'i>, key: Expr<'i>, value: Vec<Expr<'i>>) -> StructMemberNode<'i> {
        StructMemberNode {
            span,
            key: Box::new(key),
            value,
        }
    }
}

#[derive(Debug)]
pub enum Expr<'i> {
    Blob(ValueNode<'i>),
    Boolean(ValueNode<'i>),
    Clob(ExpressionsNode<'i>),
    CommentBlock(NonAnnotatedValues<'i>),
    CommentLine(NonAnnotatedValue<'i>),
    Integer(ValueNode<'i>),
    List(ExpressionsNode<'i>),
    MultilineString(ValuesNode<'i>),
    Null(ValueNode<'i>),
    QuotedString(ValueNode<'i>),
    Real(ValueNode<'i>),
    SExpr(ExpressionsNode<'i>),
    Struct(ExpressionsNode<'i>),
    StructKey(ValueNode<'i>),
    StructMember(StructMemberNode<'i>),
    Symbol(ValueNode<'i>),
    Timestamp(ValueNode<'i>),
    Newlines(NewlinesNode<'i>),
}
impl<'i> Expr<'i> {
    pub fn attach_annotations(mut self: Expr<'i>, annotations: Vec<String>) -> Expr<'i> {
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
}
