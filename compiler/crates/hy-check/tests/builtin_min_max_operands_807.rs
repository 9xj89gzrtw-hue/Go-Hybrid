//! Regression for #807: builtin call shapes for `min`, `max`, `complex`,
//! `real`, and `imag` (HYB-E2005), plus `...` spread on builtins that Go
//! does not declare variadic.
//!
//! `hyb check` returned `OK (0 diagnostics)` for `min(xs...)`,
//! `min(1, "foo")`, `complex("a", "b")`, `real("abc")`, `panic(xs...)`,
//! `clear(xs...)`, `close(ch...)`, and `make([]int, xs...)`; the emitted Go
//! then failed `go build` (`cannot use ... in call to min`, `invalid
//! argument: "foo" ... cannot be used as type int`).

use hy_check::{check_file, Verdict};
use hy_diag::Severity;
use hy_syntax::parse_file;

fn check(src: &str) -> hy_check::CheckResult {
    let ast = parse_file("t.hy", src).expect("snippet parses");
    check_file(&ast)
}

fn errors(r: &hy_check::CheckResult) -> Vec<(String, String)> {
    r.diagnostics
        .iter()
        .filter(|d| d.severity == Severity::Error)
        .map(|d| (d.code.clone(), d.message.clone()))
        .collect()
}

fn has_e2005(r: &hy_check::CheckResult) -> bool {
    errors(r).iter().any(|(code, _)| code == "HYB-E2005")
}

#[test]
fn min_max_spread_rejects() {
    let r = check(
        "fn main() {\n    xs := []int{1, 2}\n    _ = min(xs...)\n    _ = max(xs...)\n}\n",
    );
    assert!(
        errors(&r).len() >= 2,
        "both spreads must reject, got {:?}",
        errors(&r)
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn min_max_mixed_ordered_types_reject() {
    let r = check("fn main() {\n    _ = min(1, \"foo\")\n}\n");
    assert!(has_e2005(&r), "got {:?}", errors(&r));
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn complex_string_operands_reject() {
    let r = check("fn main() {\n    _ = complex(\"a\", \"b\")\n}\n");
    assert!(has_e2005(&r), "got {:?}", errors(&r));
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn real_string_operand_rejects() {
    let r = check("fn main() {\n    _ = real(\"abc\")\n}\n");
    assert!(has_e2005(&r), "got {:?}", errors(&r));
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn spread_on_non_variadic_builtins_rejects() {
    let r = check(
        "fn main() {\n    xs := []int{1}\n    ch := make(chan int)\n    panic(xs...)\n    clear(xs...)\n    close(ch...)\n    _ = make([]int, xs...)\n}\n",
    );
    let messages = errors(&r);
    assert!(
        messages.iter().filter(|(c, _)| c == "HYB-E2005").count() >= 4,
        "every spread must reject, got {messages:?}"
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn valid_builtin_calls_stay_accepted() {
    let r = check(
        "fn main() {\n    _ = min(1, 2)\n    _ = max(\"a\", \"b\")\n    z := complex(1.0, 2.0)\n    _ = real(z)\n    _ = imag(z)\n    xs := []int{1}\n    ys := []int{2}\n    _ = append(xs, ys...)\n}\n",
    );
    assert!(
        errors(&r).is_empty(),
        "valid builtin shapes must stay accepted, got {:?}",
        errors(&r)
    );
}
