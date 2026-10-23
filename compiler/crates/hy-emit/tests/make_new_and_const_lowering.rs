use hy_emit::emit_go;
use hy_syntax::parse_file;

fn emit(src: &str) -> Result<String, hy_emit::EmitError> {
    let ast = parse_file("inline.hy", src)
        .unwrap_or_else(|diags| panic!("must parse (parser contract): {:?}", diags));
    emit_go(&ast)
}

#[test]
fn emit_make_with_type_alias_and_parenthesized_type() {
    let go = emit(
        "type int_slice = []int\ntype str_map = map[string]int\nfn main() {\n    s := make((int_slice), 4)\n    c := make((chan int), 1)\n    m := (make)(str_map)\n    fmt.println(len(s), len(c), len(m))\n}\n",
    )
    .expect("must emit");
    assert!(go.contains("make(int_slice, 4)"), "got:\n{}", go);
    assert!(go.contains("make(chan int, 1)"), "got:\n{}", go);
    assert!(go.contains("make(str_map)"), "got:\n{}", go);
}

#[test]
fn emit_new_slice_chan_and_spec_primitive() {
    let go = emit(
        "type str_map = map[string]int\nfn main() {\n    ps := new([]int)\n    pm := new((str_map))\n    pc := new((chan int))\n    pu := new(u8)\n    fmt.println(ps, pm, pc, pu)\n}\n",
    )
    .expect("must emit");
    assert!(go.contains("new([]int)"), "got:\n{}", go);
    assert!(!go.contains("new([]int{})"), "got:\n{}", go);
    assert!(go.contains("new(str_map)"), "got:\n{}", go);
    assert!(go.contains("new(chan int)"), "got:\n{}", go);
    assert!(go.contains("new(uint8)"), "got:\n{}", go);
}

#[test]
fn emit_bare_chan_value_expression_refused() {
    let err = emit("fn main() {\n    c := chan int\n    fmt.println(c)\n}\n")
        .expect_err("bare chan T outside make/new must fail emit");
    assert!(
        err.message.contains("chan") || err.message.contains("make"),
        "unexpected error: {:?}",
        err
    );
}

#[test]
fn emit_parenthesized_composite_const_rewritten_to_var() {
    let go = emit(
        "const top_items = ([]int{1, 2})\nfn main() {\n    const local_arr = ([10, 20])\n    fmt.println(top_items[0], local_arr[0])\n}\n",
    )
    .expect("must emit");
    assert!(
        go.contains("var top_items ="),
        "expected var top_items =, got:\n{}",
        go
    );
    assert!(
        go.contains("var local_arr ="),
        "expected var local_arr =, got:\n{}",
        go
    );
}

#[test]
fn emit_new_on_array_literal_keeps_the_array_type() {
    // #1356: `new([4]u8{})` is the zero value of a pointer-to-array, not the
    // zero value of a slice of arrays.
    let go = emit("fn main() {\n    pa := new([4]u8{})\n    fmt.println(pa)\n}\n").expect("must emit");
    assert!(go.contains("new([4]uint8)"), "got:\n{}", go);
    assert!(!go.contains("new([][4]uint8)"), "got:\n{}", go);
}

#[test]
fn emit_make_on_parenthesized_empty_slice_literal_stays_a_slice() {
    // #1356: the array guard must look at the un-parenthesized target.
    let go = emit("fn main() {\n    s := make(([]int{}), 0)\n    fmt.println(len(s))\n}\n")
        .expect("must emit");
    assert!(go.contains("make([]int, 0)"), "got:\n{}", go);
}
