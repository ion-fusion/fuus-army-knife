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
pub struct ValueNode<'i> {
    pub span: Span<'i>,
    pub annotations: Vec<String>,
    pub value: String,
}
impl<'i> ValueNode<'i> {
    pub fn new(span: Span<'i>, annotations: Vec<String>, value: String) -> ValueNode<'i> {
        ValueNode {
            span,
            annotations,
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

#[derive(Debug)]
pub struct ExpressionsNode<'i> {
    pub span: Span<'i>,
    pub annotations: Vec<String>,
    pub value: Vec<Expr<'i>>,
}

#[derive(Debug)]
pub struct StructMember<'i> {
    pub span: Span<'i>,
    pub key: String,
    pub value: Box<Expr<'i>>,
}

#[derive(Debug)]
pub struct StructNode<'i> {
    pub span: Span<'i>,
    pub annotations: Vec<String>,
    pub value: Vec<StructMember<'i>>,
}

#[derive(Debug)]
pub enum Expr<'i> {
    Blob(ValueNode<'i>),
    Boolean(ValueNode<'i>),
    Clob(ValuesNode<'i>),
    Comment(NonAnnotatedValue<'i>),
    Integer(ValueNode<'i>),
    List(ExpressionsNode<'i>),
    MultilineString(ValuesNode<'i>),
    Null(ValueNode<'i>),
    QuotedString(ValueNode<'i>),
    Real(ValueNode<'i>),
    SExpr(ExpressionsNode<'i>),
    Struct(StructNode<'i>),
    Symbol(ValueNode<'i>),
    Timestamp(ValueNode<'i>),
    Whitespace(NonAnnotatedValue<'i>),
}
