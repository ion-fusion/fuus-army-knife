// Copyright Ion Fusion contributors. All Rights Reserved.
use regex::Regex;

pub fn count_newlines(input: &str) -> usize {
    let newline_regex = Regex::new(r"\r\n?|\n").unwrap();
    newline_regex.find_iter(input).count()
}

pub fn repeat(chr: char, count: usize) -> String {
    (0..count).map(|_| chr).collect::<String>()
}

#[cfg(test)]
#[test]
fn test_repeat() {
    assert_eq!("", repeat('0', 0));
    assert_eq!("1", repeat('1', 1));
    assert_eq!("22", repeat('2', 2));
    assert_eq!("333", repeat('3', 3));
    assert_eq!("####", repeat('#', 4));
}

pub fn find_cursor_pos(value: &str) -> usize {
    match value.rfind('\n') {
        Some(index) => (value.len() - index) - 1,
        None => value.len(),
    }
}

#[cfg(test)]
#[test]
fn test_find_cursor_pos() {
    assert_eq!(2, find_cursor_pos("  "));
    assert_eq!(2, find_cursor_pos("   \n  "));
    assert_eq!(0, find_cursor_pos("   \n"));
}

pub fn already_has_whitespace_before_cursor(value: &str) -> bool {
    let value = value.as_bytes();
    let index = (value.len() as isize) - 1;
    return index > 0 && (value[index as usize] == b' ' || value[index as usize] == b'\n');
}

#[cfg(test)]
#[test]
fn test_already_has_whitespace_before_cursor() {
    assert!(!already_has_whitespace_before_cursor(""));
    assert!(already_has_whitespace_before_cursor("  "));
    assert!(already_has_whitespace_before_cursor("foo\n"));
    assert!(already_has_whitespace_before_cursor("foo "));
    assert!(!already_has_whitespace_before_cursor("foo"));
}

pub fn indent_len(value: &str) -> usize {
    let mut count = 0;
    for chr in value.chars() {
        if chr != ' ' && chr != '\t' {
            break;
        }
        count += 1;
    }
    return count;
}

#[cfg(test)]
#[test]
fn test_indent_len() {
    assert_eq!(0, indent_len("foo"));
    assert_eq!(1, indent_len(" foo"));
    assert_eq!(2, indent_len("\t foo"));
    assert_eq!(2, indent_len("  foo\nfoo"));
}

pub fn min_indent_len(value: &str) -> usize {
    value
        .lines()
        .filter(|line| line.trim().len() > 0)
        .map(|line| indent_len(line))
        .min()
        .unwrap_or(0)
}

#[cfg(test)]
#[test]
fn test_min_indent_len() {
    assert_eq!(0, min_indent_len("foo"));
    assert_eq!(0, min_indent_len("\n"));
    assert_eq!(0, min_indent_len("\nfoo"));
    assert_eq!(2, min_indent_len("\n  foo\n  bar"));
    assert_eq!(1, min_indent_len("  \n foo\n  bar"));
    assert_eq!(0, min_indent_len("foo  \n foo\n  bar"));
}

pub fn trim_indent(value: &str) -> String {
    let min_indent = min_indent_len(value);
    let mut output = String::new();
    let mut ws = 0;
    for chr in value.chars() {
        if chr == ' ' || chr == '\t' {
            ws += 1;
            if ws > min_indent {
                output.push(chr);
            }
        } else {
            if chr == '\n' {
                ws = 0;
            }
            output.push(chr);
        }
    }
    output
}

#[cfg(test)]
#[test]
fn test_trim_indent() {
    assert_eq!("foo\nbar", &trim_indent("foo\nbar"));
    assert_eq!("foo\nbar", &trim_indent(" foo\n bar"));
    assert_eq!("foo\nbar", &trim_indent("  foo\n  bar"));
    assert_eq!("foo\n bar", &trim_indent(" foo\n  bar"));
    assert_eq!(
        "\nfoo\n  bar\nbaz\n",
        &trim_indent("\n  foo\n    bar\n  baz\n")
    );
}

pub fn format_indented_multiline(value: &str, continuation_indent: usize) -> String {
    let indent = repeat(' ', continuation_indent);
    let mut output = String::new();
    let mut indent_next = false;
    for chr in value.chars() {
        if indent_next && chr != '\n' {
            output.push_str(&indent);
            indent_next = false;
        }

        output.push(chr);
        if chr == '\n' {
            indent_next = true;
        }
    }
    output
}

#[cfg(test)]
#[test]
fn test_format_indented_multiline() {
    assert_eq!("foo", &format_indented_multiline("foo", 3));
    assert_eq!("foo\n   bar", &format_indented_multiline("foo\nbar", 3));
    assert_eq!(
        "foo\n\n\n   bar",
        &format_indented_multiline("foo\n\n\nbar", 3)
    );
    assert_eq!(
        "foo\n   bar\n     baz\n   bin",
        &format_indented_multiline("foo\nbar\n  baz\nbin", 3)
    );
}

pub fn last_is_one_of(value: &str, chars: &[char]) -> bool {
    if let Some(last) = value.chars().last() {
        for chr in chars {
            if last == *chr {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
#[test]
fn test_last_is_one_of() {
    assert!(!last_is_one_of("", &['!']));
    assert!(last_is_one_of("!", &['!']));
    assert!(!last_is_one_of("!#", &['!']));
    assert!(last_is_one_of("!#", &['!', '#']));
}
