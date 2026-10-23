//! Regression for #1297: `none` in a value position (typed `var` initializer,
//! keyed struct-literal field, assignment to an `option[T]` binding) passed
//! `hyb check` and then the emitter refused it. Every one of these positions
//! declares its target type, so the emitter can lower `none` to the prelude
//! option's zero value (`option[T]{}`, `ok: false`) instead of failing.

use hy_emit::emit_go;
use hy_syntax::parse_file;

fn emit(src: &str) -> Result<String, hy_emit::EmitError> {
    let ast = parse_file("inline.hy", src)
        .unwrap_or_else(|diags| panic!("must parse (parser contract): {:?}", diags));
    emit_go(&ast)
}

#[test]
fn emit_none_as_typed_var_initializer() {
    let go = emit(
        "fn main() {\n    var o option[int] = none\n    fmt.println(o.is_none())\n}\n",
    )
    .expect("`var o option[int] = none` must emit");
    assert!(go.contains("var o option[int] = option[int]{}"), "got:\n{}", go);
}

#[test]
fn emit_none_as_keyed_struct_field() {
    let go = emit(
        "struct wrap {\n    inner option[int]\n}\n\nfn main() {\n    w := wrap{inner: none}\n    fmt.println(w.inner.is_none())\n}\n",
    )
    .expect("`wrap{inner: none}` must emit");
    assert!(go.contains("inner: option[int]{}"), "got:\n{}", go);
}

#[test]
fn emit_none_assigned_to_option_binding() {
    let go = emit(
        "fn main() {\n    var o option[int] = some(1)\n    o = none\n    fmt.println(o.is_none())\n}\n",
    )
    .expect("`o = none` must emit");
    assert!(go.contains("o = option[int]{}"), "got:\n{}", go);
}
