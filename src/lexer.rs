// Copyright Ion Fusion contributors. All Rights Reserved.
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "grammar.pest"]
pub struct FusionLexer;

pub type FPair<'i> = Pair<'i, Rule>;
pub type FPairs<'i> = Pairs<'i, Rule>;

#[cfg(test)]
mod lexer_tests {
    use super::{FusionLexer, Rule};
    use pest::Parser;

    macro_rules! test_success {
        ($rule:ident, $input:expr) => {
            test_success!($rule, $input, $input);
        };
        ($rule:ident, $expected:expr, $input:expr) => {
            println!("Expect success: {} -> {}", $input, $expected);
            let result = FusionLexer::parse(Rule::$rule, $input);
            if let Err(error) = result {
                assert!(false, "Error: {}", error);
            } else {
                assert_eq!($expected, result.ok().unwrap().as_str());
            }
        };
    }

    macro_rules! test_fail {
        ($rule:ident, $input:expr) => {
            println!("Expect fail: {}", $input);
            let result = FusionLexer::parse(Rule::$rule, $input);
            if let Ok(success) = result {
                assert!(false, "Expected failure, but got: {:#?}", success);
            }
        };
    }

    #[test]
    fn null() {
        let success_cases = vec![
            "null",
            "null.blob",
            "null.clob",
            "null.bool",
            "null.int",
            "null.list",
            "null.decimal",
            "null.float",
            "null.symbol",
            "null.string",
            "null.timestamp",
            "null.sexp",
            "null.struct",
        ];
        for case in &success_cases {
            test_success!(null, *case);
        }

        let failure_cases = vec!["nul", "null.", "'null.list'", "'null'"];
        for case in &failure_cases {
            test_fail!(null, *case);
        }
    }

    #[test]
    fn annotations() {
        test_success!(annotations, "foo::");
        test_success!(annotations, "'foo'::");
        test_success!(annotations, "'foo bar'::");
        test_success!(annotations, "foo::bar::");
        test_success!(annotations, "foo :: bar ::");
        test_fail!(annotations, "foo:");
        test_fail!(annotations, "foo");
        test_fail!(annotations, "null.symbol::");
    }

    #[test]
    fn boolean() {
        test_success!(boolean, "true");
        test_success!(boolean, "false");
        test_fail!(boolean, "TRUE");
        test_fail!(boolean, "'true'");
    }

    #[test]
    fn integer() {
        test_success!(integer, "0");
        test_success!(integer, "-0");
        test_success!(integer, "123");
        test_success!(integer, "-123");
        test_success!(integer, "0xBeef");
        test_success!(integer, "0b0101");
        test_success!(integer, "1_2_3");
        test_success!(integer, "0xFA_CE");
        test_success!(integer, "0b10_10_10");
        test_fail!(integer, "+1");
        test_fail!(integer, "0123");
        test_fail!(integer, "1_");
        test_fail!(integer, "1__2");
        test_fail!(integer, "0x_12");
        test_fail!(integer, "_1");
    }

    #[test]
    fn real() {
        test_success!(real, "0.123");
        test_success!(real, "-0.12e4");
        test_success!(real, "-0.12d4");
        test_success!(real, "0E0");
        test_success!(real, "0D0");
        test_success!(real, "0.");
        test_success!(real, "-0e0");
        test_success!(real, "-0d0");
        test_success!(real, "-0.");
        test_success!(real, "-0d-1");
        test_success!(real, "123_456.789_012");
        test_fail!(real, "123_._456");
        test_fail!(real, "12__34.56");
        test_fail!(real, "123.456_");
        test_fail!(real, "-_123.456");
        test_fail!(real, "_123.456");
    }

    #[test]
    fn symbol() {
        let success_cases = vec![
            "'myVar2'",
            "myVar2",
            "myvar2",
            "'hi ho'",
            "'\\'ahoy\\''",
            "''",
            "foo_baz",
            "+",
            "-",
        ];
        for case in &success_cases {
            test_success!(symbol, *case);
        }

        let failure_cases = vec!["", " ", "\t", "0", "0Foo", "1Foo"];
        for case in &failure_cases {
            test_fail!(symbol, *case);
        }
    }

    #[test]
    fn timestamp() {
        test_success!(timestamp, "2007-02-23T12:14Z");
        test_success!(timestamp, "2007-02-23T12:14:33.079-08:00");
        test_success!(timestamp, "2007-02-23T20:14:33.079Z");
        test_success!(timestamp, "2007-02-23T20:14:33.079+00:00");
        test_success!(timestamp, "2007-02-23T20:14:33.079-00:00");
        test_success!(timestamp, "2007-01-01T00:00-00:00");
        test_success!(timestamp, "2007-01-01");
        test_success!(timestamp, "2007-01-01T");
        test_success!(timestamp, "2007-01T");
        test_success!(timestamp, "2007T");
        test_success!(timestamp, "2007-02-23");
        test_success!(timestamp, "2007-02-23T00:00Z");
        test_success!(timestamp, "2007-02-23T00:00+00:00");
        test_success!(timestamp, "2007-02-23T00:00:00-00:00");
        test_fail!(timestamp, "2007");
        test_fail!(timestamp, "2007-01");
        test_fail!(timestamp, "2007-02-23T20:14:33.Z");
    }

    #[test]
    fn string() {
        test_success!(string, "\"\"");
        test_success!(string, "\" my string \"");
        test_success!(string, "\"\\\"\"");
        test_success!(string, "\"\\uABCD\"");
        test_success!(string, "\"\\n\"");
        test_success!(string, "'''foo'''");
        test_success!(string, "'''foo\nbar'''");
        test_fail!(string, "\"");
        test_fail!(string, "\"foo\nbar\"");
        test_fail!(string, "\"\\\"");
        test_fail!(string, "'''foo");
    }

    #[test]
    fn blob() {
        test_success!(blob, "{{\n +AB/ \n}}");
        test_success!(blob, "{{ VG8gaW5maW5pdHkuLi4gYW5kIGJleW9uZCE= }}");
        test_fail!(blob, "{{ VG8gaW5maW5pdHkuLi4gYW5kIGJleW9uZCE= }");
    }

    #[test]
    fn clob() {
        test_success!(clob, "{{ \"foo bar\" }}");
        test_success!(clob, "{{\n '''foo'''\n     '''bar'''\n}}");
        test_fail!(clob, "{{\n// no comments\n'''foo'''\n     '''bar'''\n}}");
    }

    #[test]
    fn structure() {
        test_success!(structure, "{}");
        test_success!(structure, "{ }");
        test_success!(structure, "{foo:1}");
        test_success!(structure, "{ foo:1, bar:2 }");
        test_success!(structure, "{ foo: 1, bar: 2, }");
        test_success!(structure, "{ foo: 1, bar: { baz: 5 }, }");
        test_success!(structure, "{ foo: annotated::1, bar: { baz: 5 }, }");
        test_success!(structure, "{ foo: \"1 2 3\", bar: { baz: 5 }, }");
        test_success!(structure, "{ foo: \"1 2 3\", bar: (+ 1 2) }");
        test_success!(structure, "{ foo: \"1 2 3\", bar: annotated::[1, 2] }");
        test_success!(structure, "{ foo: 2007-01-01 }");
        test_fail!(structure, "{");
        test_fail!(structure, "{ foo: 2007-01-01 ");
        test_fail!(structure, "{ foo 2007-01-01 }");
        test_fail!(structure, "{ 5 }");
        test_fail!(structure, "{ , baz: 5 }");
        test_fail!(structure, "{ annotated::foo: 1 }");
    }

    #[test]
    fn list() {
        test_success!(list, "[]");
        test_success!(list, "[ ]");
        test_success!(list, "[1, 2, 3]");
        test_success!(list, "[1, 2, 3,]");
        test_success!(list, "[1, 2, [b]]");
        test_fail!(list, "[1, , 2]");
        test_fail!(list, "[");
        test_fail!(list, "[[b]");
    }

    #[test]
    fn sexpr() {
        test_success!(sexpr, "()");
        test_success!(sexpr, "(cons 1 2)");
        test_success!(sexpr, "([hello][there])");
        test_success!(sexpr, "(a+-b)");
        test_success!(sexpr, "(a.b;)");
        test_fail!(sexpr, "(");
    }

    #[test]
    fn comment() {
        test_success!(line_comment, "// test\n");
        test_success!(line_comment, "// test\n", "// test\nfoo");
        test_success!(line_comment, "//test\n", "//test\nfoo");
        test_success!(line_comment, "///test\n", "///test\nfoo");
        test_success!(block_comment, "/* test */");
        test_success!(block_comment, "/* test\ntest */");
    }
}
