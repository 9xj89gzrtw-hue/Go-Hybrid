//! Issue #777 — package-level user declarations that collide with the
//! emitter's auto-generated prelude.
//!
//! The emitter injects `type option`, `func some`, `type set`, and
//! `func hy_pop` into the generated Go when a program triggers the
//! option/set/slice surface. A user declaration with one of those names lands
//! in the same Go package block, so `hyb check` accepted the program while
//! `go build` refused it (`some redeclared in this block`). The checker now
//! rejects the colliding declaration as HYB-E1002 — but only when the
//! prelude is actually injected, so a user `fn set` with no set surface
//! stays accepted.

use hy_check::check_file;
use hy_syntax::parse_file;

fn codes(src: &str) -> Vec<String> {
    let ast = parse_file("test.hy", src).expect("parses");
    check_file(&ast)
        .diagnostics
        .into_iter()
        .map(|d| d.code)
        .collect()
}

fn assert_code(src: &str, want: &str) {
    let cs = codes(src);
    assert!(
        !cs.is_empty() && cs.iter().all(|c| c == want),
        "expected only [{want}], got {cs:?} for:\n{src}"
    );
}

fn assert_ok(src: &str) {
    let cs = codes(src);
    assert!(
        cs.is_empty(),
        "expected 0 diagnostics, got {cs:?} for:\n{src}"
    );
}

// ---------------------------------------------------------------------------
// A — a user function named `some` with option in a signature
// ---------------------------------------------------------------------------

#[test]
fn user_fn_some_colliding_with_the_option_prelude_is_rejected() {
    assert_code(
        "\
fn some(x int) int {
    return x + 1
}

fn has(o option[int]) bool {
    return o.is_some()
}

fn main() {
    n := some(1)
    fmt.println(\"{n}\")
}
",
        "HYB-E1002",
    );
}

// ---------------------------------------------------------------------------
// B — a user type named `option` with option in a signature
// ---------------------------------------------------------------------------

#[test]
fn user_struct_option_colliding_with_the_option_prelude_is_rejected() {
    assert_code(
        "\
struct option {
    n int
}

fn has(o option[int]) bool {
    return o.is_some()
}

fn main() {
    v := option{n: 1}
    fmt.println(\"{v.n}\")
}
",
        "HYB-E1002",
    );
}

// ---------------------------------------------------------------------------
// The other names the prelude declares at package scope
// ---------------------------------------------------------------------------

#[test]
fn user_fn_set_with_a_set_surface_is_rejected() {
    // `set[int]{}` is the emitter's `set[` trigger; the user `fn set` would
    // collide with `type set` in the generated package block.
    assert_code(
        "\
fn set(x int) int {
    return x
}

fn main() {
    s := set[int]{}
    s.insert(1)
    fmt.println(\"{len(s)}\")
}
",
        "HYB-E1002",
    );
}

#[test]
fn user_fn_hy_pop_colliding_with_the_slice_prelude_is_rejected() {
    assert_code(
        "\
fn hy_pop(x int) int {
    return x
}

fn main() {
    fmt.println(\"{hy_pop(1)}\")
}
",
        "HYB-E1002",
    );
}

#[test]
fn user_const_and_type_alias_with_reserved_names_are_rejected() {
    assert_code(
        "\
const option = 1

fn has(o option[int]) bool {
    return o.is_some()
}

fn main() {
    fmt.println(\"{option}\")
}
",
        "HYB-E1002",
    );
    assert_code(
        "\
type some = int

fn has(o option[int]) bool {
    return o.is_some()
}

fn main() {
    fmt.println(\"ok\")
}
",
        "HYB-E1002",
    );
}

// ---------------------------------------------------------------------------
// No over-rejection: a reserved *word* is fine while its prelude is dormant
// ---------------------------------------------------------------------------

#[test]
fn fn_set_without_a_set_surface_is_allowed() {
    // The existing corpus pattern: `set` as an ordinary helper function.
    assert_ok(
        "\
fn set(dst []map[string]int, v map[string]int) {
    dst[0] = v
}

fn main() {
    dst := make([]map[string]int, 1)
    set(dst, map[string]int{\"k\": 1})
    fmt.println(\"{dst[0][\"k\"]}\")
}
",
    );
}

#[test]
fn struct_set_used_as_a_plain_struct_is_allowed() {
    // `set{n: 1}` emits no `set[`, so the set prelude is dormant.
    assert_ok(
        "\
struct set {
    n int
}

fn main() {
    v := set{n: 1}
    fmt.println(\"{v.n}\")
}
",
    );
}

#[test]
fn method_named_some_is_allowed() {
    assert_ok(
        "\
struct s {
    n int
}

fn (x s) some() int {
    return x.n
}

fn main() {
    v := s{n: 1}
    fmt.println(\"{v.some()}\")
}
",
    );
}

#[test]
fn local_named_some_is_allowed() {
    assert_ok(
        "\
fn main() {
    some := 1
    fmt.println(\"{some}\")
}
",
    );
}

// ---------------------------------------------------------------------------
// Issue #1035 — Go builtins called by the prelude body. A package-level
// declaration shadows the builtin for the whole Go package, including the
// auto-generated prelude. `panic` is the silent one: unwrap(none) returns the
// zero value instead of panicking with HY-PANIC/unwrap-none.
// ---------------------------------------------------------------------------

#[test]
fn user_fn_panic_with_option_surface_is_rejected() {
    assert_code(
        "\
fn panic(msg string) {
    fmt.println(msg)
}

fn main() {
    var o option[int]
    fmt.println(\"{o.is_some()}\")
}
",
        "HYB-E1002",
    );
}

#[test]
fn user_fn_len_with_set_surface_is_rejected() {
    assert_code(
        "\
fn len(xs []int) int {
    return 0
}

fn main() {
    s := set[int]{}
    s.insert(1)
    fmt.println(\"{s.contains(1)}\")
}
",
        "HYB-E1002",
    );
}

#[test]
fn user_fn_make_with_set_surface_is_rejected() {
    assert_code(
        "\
fn make(x int) int {
    return x
}

fn main() {
    s := set[int]{}
    s.insert(1)
    fmt.println(\"{s.contains(1)}\")
}
",
        "HYB-E1002",
    );
}

#[test]
fn user_fn_delete_with_set_surface_is_rejected() {
    assert_code(
        "\
fn delete(x int) int {
    return x
}

fn main() {
    s := set[int]{}
    s.insert(1)
    fmt.println(\"{s.contains(1)}\")
}
",
        "HYB-E1002",
    );
}

#[test]
fn user_fn_len_with_slice_prelude_is_rejected() {
    assert_code(
        "\
fn len(xs []int) int {
    return 0
}

fn main() {
    xs := []int{1, 2}
    n := xs.pop()
    fmt.println(\"{n}\")
}
",
        "HYB-E1002",
    );
}

// ---------------------------------------------------------------------------
// No over-rejection: the builtin is only reserved while its prelude is active,
// and only at package level.
// ---------------------------------------------------------------------------

#[test]
fn fn_panic_without_the_option_surface_is_allowed() {
    assert_ok(
        "\
fn panic(msg string) {
    fmt.println(msg)
}

fn main() {
    panic(\"hello\")
}
",
    );
}

#[test]
fn fn_make_with_only_the_option_surface_is_allowed() {
    // The option prelude calls `panic`, not `make`.
    assert_ok(
        "\
fn make(x int) int {
    return x
}

fn main() {
    var o option[int]
    n := make(1)
    fmt.println(\"{n}\")
    fmt.println(\"{o.is_some()}\")
}
",
    );
}

#[test]
fn fn_len_without_a_set_or_slice_surface_is_allowed() {
    assert_ok(
        "\
fn len(x int) int {
    return x
}

fn main() {
    n := len(1)
    fmt.println(\"{n}\")
}
",
    );
}

#[test]
fn local_named_panic_is_allowed() {
    assert_ok(
        "\
fn main() {
    panic := 1
    fmt.println(\"{panic}\")
}
",
    );
}
