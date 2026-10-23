//! Regression for #1358: a multi-return call through a `fn`-typed struct field
//! (`_, rx := h.f()` where `f fn() int <-chan int`) must keep each result's
//! channel direction, so `close(rx)` on a receive-only channel is rejected with
//! HYB-E3009 instead of failing later in `go build`.

use hy_check::{check_file, Verdict};
use hy_diag::Severity;
use hy_syntax::parse_file;

fn check(src: &str) -> hy_check::CheckResult {
    let ast = parse_file("t.hy", src).expect("snippet parses");
    check_file(&ast)
}

fn first_code(r: &hy_check::CheckResult) -> Option<String> {
    r.diagnostics
        .iter()
        .find(|d| d.severity == Severity::Error)
        .map(|d| d.code.clone())
}

#[test]
fn close_recv_only_result_of_fn_field_call_rejects_e3009() {
    let r = check(
        "struct holder {\n    f fn() int <-chan int\n}\n\nfn main() {\n    ch := make(chan int, 1)\n    h := holder{f: fn() (int, <-chan int) { return 1, ch }}\n    _, rx := h.f()\n    close(rx)\n}\n",
    );
    assert_eq!(
        first_code(&r),
        Some("HYB-E3009".to_string()),
        "expected HYB-E3009, got {:?}",
        r.diagnostics
            .iter()
            .map(|d| (&d.code, &d.message))
            .collect::<Vec<_>>()
    );
    assert_eq!(r.verdict, Verdict::Reject);
}

#[test]
fn close_bidirectional_result_of_fn_field_call_stays_accepted() {
    let r = check(
        "struct holder {\n    f fn() int chan int\n}\n\nfn main() {\n    ch := make(chan int, 1)\n    h := holder{f: fn() (int, chan int) { return 1, ch }}\n    _, tx := h.f()\n    close(tx)\n}\n",
    );
    assert!(
        !r.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error),
        "bidirectional channel stays closable, got {:?}",
        r.diagnostics
            .iter()
            .map(|d| (&d.code, &d.message))
            .collect::<Vec<_>>()
    );
}
