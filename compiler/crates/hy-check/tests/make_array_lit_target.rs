//! Regression for #1356: `make([N]T{}, …)` is an array composite literal, not a
//! slice type, and must be rejected with HYB-E2005 at check time instead of
//! passing `hyb check` and only failing later in the emitter.

use hy_check::{check_file, Verdict};
use hy_diag::Severity;
use hy_syntax::parse_file;

fn check(src: &str) -> hy_check::CheckResult {
    let ast = parse_file("test.hy", src).expect("test snippet must parse");
    check_file(&ast)
}

fn error_codes(r: &hy_check::CheckResult) -> Vec<String> {
    r.diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| d.code.clone())
        .collect()
}

fn assert_rejected_with_e2005(src: &str) {
    let r = check(src);
    let cs = error_codes(&r);
    assert!(
        cs.iter().any(|c| c == "HYB-E2005"),
        "expected HYB-E2005 for `{}`, got {:?}",
        src,
        cs
    );
    assert_eq!(r.verdict, Verdict::Reject, "expected reject for `{}`", src);
}

#[test]
fn make_on_empty_array_literal_is_rejected() {
    assert_rejected_with_e2005("fn main() {\n    s := make([4]int{}, 4)\n    fmt.println(len(s))\n}\n");
}

#[test]
fn make_on_inferred_length_array_literal_is_rejected() {
    assert_rejected_with_e2005(
        "fn main() {\n    s := make([...]int{}, 4)\n    fmt.println(len(s))\n}\n",
    );
}

#[test]
fn make_on_parenthesized_array_literal_is_rejected() {
    assert_rejected_with_e2005(
        "fn main() {\n    s := make(([4]int{}), 4)\n    fmt.println(len(s))\n}\n",
    );
}

#[test]
fn make_on_empty_slice_literal_stays_accepted() {
    let r = check("fn main() {\n    s := make([]int{}, 0)\n    fmt.println(len(s))\n}\n");
    let cs = error_codes(&r);
    assert!(
        !cs.iter().any(|c| c == "HYB-E2005"),
        "empty slice literal is a legal make target, got {:?}",
        cs
    );
}
