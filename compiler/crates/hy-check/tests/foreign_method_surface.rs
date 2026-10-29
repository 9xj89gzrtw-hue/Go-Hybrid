//! Regression: the checker's foreign METHOD surface must not bless a name the
//! emitter cannot lower and the vendored Go binding does not ship (#2200).
//!
//! `hyb check` used to accept `doc.get("k")`, `doc.remove("k")` and
//! `doc.to_string_pretty()` on a `serde_json.value` handle, because
//! `foreign::method_mode` listed those three names alongside the real ones.
//! None of them is a method on `jsonbinding.Value` — `get`/`remove` are not
//! shipped at all, and `to_string_pretty` is a PACKAGE function
//! (`serde_json.to_string_pretty(doc)`). Every one of them therefore passed
//! `hyb check` and then died in the toolchain:
//!
//! ```text
//! ./main.go:17:20: doc.Get undefined (type jsonbinding.Value has no field or method Get)
//! ./main.go:17:23: doc.Remove undefined (type jsonbinding.Value has no field or method Remove)
//! ./main.go:17:20: doc.ToStringPretty undefined (type jsonbinding.Value has no field or method ToStringPretty)
//! ```
//!
//! That violates GUARANTEES.md row 13 (an accepted program must emit Go that
//! survives `go build`). These tests pin the rejection of the phantom names and
//! the acceptance of the five real ones, so the table cannot drift back.

use hy_check::{check_file, Verdict};

fn check(src: &str) -> hy_check::CheckResult {
    let ast = hy_syntax::parse_file("test.hy", src).expect("test source must parse");
    check_file(&ast)
}

fn codes(r: &hy_check::CheckResult) -> Vec<String> {
    r.diagnostics.iter().map(|d| d.code.clone()).collect()
}

/// A `serde_json.value` handle, bound so the emitter/checker both see a handle.
fn program(call: &str) -> String {
    format!(
        "import \"rust/serde_json\"\n\
         fn main() {{\n\
             doc := serde_json.parse(`{{}}`)?\n\
             {call}\n\
         }}\n"
    )
}

#[test]
fn phantom_serde_json_methods_are_rejected() {
    // Each of these passed `hyb check` with 0 diagnostics before the fix and
    // then failed `go build` with `... has no field or method ...`.
    for call in [
        "v := doc.get(\"k\")?\n    fmt.println(\"{v}\")",
        "doc.remove(\"k\")?\n    fmt.println(\"ok\")",
        "s := doc.to_string_pretty()?\n    fmt.println(\"{s}\")",
    ] {
        let r = check(&program(call));
        assert_eq!(
            r.verdict,
            Verdict::Reject,
            "phantom method call must reject, got {:?}",
            r.diagnostics
        );
        assert!(
            codes(&r).iter().any(|c| c == "HYB-E1002"),
            "expected HYB-E1002, got {:?}",
            codes(&r)
        );
    }
}

#[test]
fn real_serde_json_methods_still_accept() {
    // The five methods `jsonbinding.Value` actually ships. `array`/returned
    // views and `set`/mutations are covered by the A2/A7 suites; this test only
    // guards that the surface tightening did not over-reject the surface itself.
    for call in [
        "a := doc.array(\"k\")?\n    fmt.println(\"n={len(a)}\")",
        "n := doc.int(\"k\")?\n    fmt.println(\"n={n}\")",
        "s := doc.string(\"k\")?\n    fmt.println(\"s={s}\")",
        "doc.set(\"k\", 1)?\n    fmt.println(\"ok\")",
        "doc.push_string(\"x\")?\n    fmt.println(\"ok\")",
    ] {
        let r = check(&program(call));
        assert_eq!(
            r.verdict,
            Verdict::Accept,
            "real method `{call}` must accept, got {:?}",
            r.diagnostics
        );
    }
}

#[test]
fn package_form_to_string_pretty_is_not_a_method() {
    // `to_string_pretty` is a package function; the method spelling must be
    // rejected while the package spelling keeps working (example 05's shape).
    let reject = check(&program("s := doc.to_string_pretty()?\n    fmt.println(\"{s}\")"));
    assert_eq!(reject.verdict, Verdict::Reject, "got {:?}", reject.diagnostics);

    let accept = check(
        "import \"rust/serde_json\"\n\
         fn main() {\n\
             doc := serde_json.parse(`{{}}`)?\n\
             fmt.println(serde_json.to_string_pretty(doc)?)\n\
         }\n",
    );
    assert_eq!(
        accept.verdict,
        Verdict::Accept,
        "package form must accept, got {:?}",
        accept.diagnostics
    );
}

#[test]
fn arbitrary_unknown_method_on_a_handle_is_rejected() {
    // The general shape behind issue #2200 itself, independent of the three
    // names above: ANY name outside the binding surface must fail closed.
    let r = check(&program(
        "doc.totally_fake_method(\"x\")?\n    fmt.println(\"ok\")",
    ));
    assert_eq!(r.verdict, Verdict::Reject, "got {:?}", r.diagnostics);
    assert!(
        codes(&r).iter().any(|c| c == "HYB-E1002"),
        "expected HYB-E1002, got {:?}",
        codes(&r)
    );
    // The diagnostic must name the offending method and list the real surface,
    // so the fix is actionable rather than a bare "no".
    let msg = &r.diagnostics[0].message;
    assert!(msg.contains("totally_fake_method"), "message: {msg}");
    let notes = r.diagnostics[0].notes.join(" | ");
    assert!(
        notes.contains("array, int, string, set, push_string"),
        "notes must list the real surface, got: {notes}"
    );
}

#[test]
fn view_shape_stays_fail_open() {
    // A `serde_json.view` is two different Go types depending on context: a
    // plain `[]jsonbinding.Value` slice (no methods — `len(v)` is the call
    // form) or an element `jsonbinding.Value`. The checker cannot tell them
    // apart from the shape alone, so views must NOT be rejected here or legal
    // element code breaks. Pinned so a future tightening does not over-reach.
    let r = check(&program("a := doc.array(\"k\")?\n    fmt.println(\"n={len(a)}\")"));
    assert_eq!(
        r.verdict,
        Verdict::Accept,
        "view `len` must stay accepted, got {:?}",
        r.diagnostics
    );
}
