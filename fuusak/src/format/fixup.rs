// Copyright Ion Fusion contributors. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0
use crate::ast::{CountUntilPred, Expr, NewlinesData};

pub fn fixup_ast(ast: &[Expr]) -> Vec<Expr> {
    ast.iter().cloned().map(fixup_expr).collect()
}

fn fixup_expr(mut expr: Expr) -> Expr {
    use Expr::{List, SExpr, Struct};

    expr = clear_empty(expr);
    match expr {
        SExpr(ref mut data) | List(ref mut data) | Struct(ref mut data) => {
            fixup_list(&mut data.items);
            let mut i = 0;
            while i < data.items.len() {
                let last_is_newlines = i == 0 || data.items[i - 1].is_newlines();
                match &mut data.items[i] {
                    SExpr(sub_data) | List(sub_data) | Struct(sub_data) => {
                        if fixup_list(&mut sub_data.items) && !last_is_newlines {
                            let newlines = Expr::Newlines(NewlinesData::new(sub_data.span, 1));
                            data.items.insert(i, newlines);
                            i += 1;
                        }
                    }
                    _ => {}
                }
                let fixed = fixup_expr(data.items.remove(i));
                data.items.insert(i, fixed);
                i += 1;
            }
        }
        _ => {}
    }
    expr
}

fn clear_empty(mut expr: Expr) -> Expr {
    use Expr::{List, SExpr, Struct};

    match expr {
        SExpr(ref mut data) | List(ref mut data) | Struct(ref mut data) => {
            if data.items.iter().all(Expr::is_newlines) {
                data.items.clear();
            }
        }
        _ => {}
    }
    expr
}

fn fixup_list(items: &mut Vec<Expr>) -> bool {
    let has_values = items.iter().any(Expr::is_not_comment_or_newlines);
    let things_before_newline = (&items[..]).count_until(|e| !e.is_newlines(), Expr::is_newlines);
    let should_add_preceding_newline = has_values && things_before_newline == 0;

    // Remove the very first newlines instance
    if let Some(line) = items.first()
        && line.is_newlines()
    {
        items.remove(0);
    }

    // Remove trailing newlines if there's no comment
    let len = items.len();
    if len >= 2 && !items[len - 2].is_comment_line() && items[len - 1].is_newlines() {
        items.remove(len - 1);
    }

    should_add_preceding_newline
}
