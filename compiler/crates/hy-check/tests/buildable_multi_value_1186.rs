//! Regression for #1186: the `BUILDABLE` operand-kind checks (HYB-E1002) must
//! see multi-value sources and same-scope `:=` re-declarations.
//!
//! Four gaps used to let `hyb check` accept programs that `go build` rejects:
//! multi-target assignment from a multi-value source, `:=` re-declaration that
//! changes an existing variable's type, 3+-result bindings losing their kinds,
//! and multi-value calls forwarded into function/method parameter lists.

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

fn has_e1002(r: &hy_check::CheckResult) -> bool {
    errors(r).iter().any(|(code, _)| code == "HYB-E1002")
}

#[test]
fn multi_target_assign_from_multi_value_source_rejects() {
    let r = check(
        "fn two_strings() (string, string) {\n    return \"a\", \"b\"\n}\n\nfn main() {\n    var a int\n    var b int\n    a, b = two_strings()\n    _ = a\n    _ = b\n}\n",
    );
    assert!(
        has_e1002(&r),
        "assignment from a multi-value source must be kind-checked, got {:?}",
        errors(&r)
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn same_scope_define_redeclare_type_conflict_rejects() {
    let r = check(
        "fn main() {\n    k := 1\n    k, c := \"hello\", 2\n    _ = k\n    _ = c\n}\n",
    );
    assert!(
        has_e1002(&r),
        "same-scope `:=` keeps the existing type, got {:?}",
        errors(&r)
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn three_result_binding_keeps_operand_kinds() {
    let r = check(
        "fn triple() (int, int, string) {\n    return 1, 2, \"s\"\n}\n\nfn main() {\n    x, y, z := triple()\n    var bad int = z\n    _ = x\n    _ = y\n    _ = bad\n}\n",
    );
    assert!(
        has_e1002(&r),
        "the third result must keep its `string` kind, got {:?}",
        errors(&r)
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn forwarded_multi_value_function_argument_rejects() {
    let r = check(
        "fn two_strings() (string, string) {\n    return \"a\", \"b\"\n}\n\nfn take_ints(x int, y int) {\n    _ = x\n    _ = y\n}\n\nfn main() {\n    take_ints(two_strings())\n}\n",
    );
    assert!(
        has_e1002(&r),
        "expanded results must match the parameters, got {:?}",
        errors(&r)
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn forwarded_multi_value_method_arity_rejects() {
    let r = check(
        "struct sink {\n    n int\n}\n\nfn (s sink) one(x int) {\n    _ = s.n\n    _ = x\n}\n\nfn two_ints() (int, int) {\n    return 1, 2\n}\n\nfn main() {\n    s := sink{ n: 0 }\n    s.one(two_ints())\n}\n",
    );
    assert!(
        has_e1002(&r),
        "a 1-param method cannot take a 2-result expansion, got {:?}",
        errors(&r)
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn valid_multi_value_forwarding_stays_accepted() {
    let r = check(
        "struct sink {\n    n int\n}\n\nfn (s sink) pair(x int, y int) {\n    _ = s.n\n    _ = x\n    _ = y\n}\n\nfn two_ints() (int, int) {\n    return 1, 2\n}\n\nfn take_ints(x int, y int) {\n    _ = x\n    _ = y\n}\n\nfn triple() (int, int, string) {\n    return 1, 2, \"s\"\n}\n\nfn main() {\n    var a int\n    var b int\n    a, b = two_ints()\n    k := 1\n    k, c := 2, 3\n    x, y, z := triple()\n    var s3 string = z\n    take_ints(two_ints())\n    s := sink{ n: 0 }\n    s.pair(two_ints())\n    _ = a\n    _ = b\n    _ = k\n    _ = c\n    _ = x\n    _ = y\n    _ = s3\n}\n",
    );
    assert!(
        errors(&r).is_empty(),
        "valid expansion must stay accepted, got {:?}",
        errors(&r)
    );
}
