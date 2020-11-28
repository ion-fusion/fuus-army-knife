// Copyright Ion Fusion contributors. All Rights Reserved.
use crate::ist::*;

pub fn fixup_ist(ist: &IntermediateSyntaxTree) -> IntermediateSyntaxTree {
    IntermediateSyntaxTree::new(fixup_exprs(&ist.expressions))
}

fn fixup_exprs(exprs: &[IExpr]) -> Vec<IExpr> {
    exprs.iter().cloned().map(|expr| fixup_expr(expr)).collect()
}

fn fixup_expr(mut expr: IExpr) -> IExpr {
    use IExpr::*;

    expr = clear_empty(expr);
    match expr {
        SExpr(ref mut data) | List(ref mut data) | Struct(ref mut data) => {
            drop(fixup_list(&mut data.items));
            let mut i = 0;
            while i < data.items.len() {
                let last_is_newlines = i == 0 || data.items[i - 1].is_newlines();
                match &mut data.items[i] {
                    SExpr(ref mut sub_data) | List(ref mut sub_data) | Struct(ref mut sub_data) => {
                        if fixup_list(&mut sub_data.items) && !last_is_newlines {
                            let newlines = IExpr::Newlines(NewlinesData::new(sub_data.span, 1));
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

fn clear_empty(mut expr: IExpr) -> IExpr {
    use IExpr::*;

    match expr {
        SExpr(ref mut data) | List(ref mut data) | Struct(ref mut data) => {
            if data.items.iter().all(|item| item.is_newlines()) {
                data.items.clear();
            }
        }
        _ => {}
    }
    expr
}

fn fixup_list(items: &mut Vec<IExpr>) -> bool {
    let has_values = items.iter().any(|item| item.is_not_comment_or_newlines());
    let things_before_newline = (&items[..]).count_until(|e| !e.is_newlines(), |e| e.is_newlines());
    let should_add_preceding_newline = has_values && things_before_newline == 0;

    // Remove the very first newlines instance
    for i in 0..items.len() {
        if items[i].is_newlines() {
            items.remove(i);
        }
        break;
    }
    // Remove trailing newlines if there's no comment
    let len = items.len();
    if len >= 2 {
        if !items[len - 2].is_comment_line() && items[len - 1].is_newlines() {
            items.remove(len - 1);
        }
    }

    should_add_preceding_newline
}
