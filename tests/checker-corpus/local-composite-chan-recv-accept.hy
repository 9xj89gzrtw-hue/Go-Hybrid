// ============================================================================
// local-composite-chan-recv-accept — Checker corpus (2026-10-31): the
// accept-side companion of local-composite-chan-recv-reject-e2008 — a local
// bound from a composite of BIDIRECTIONAL channels still receives fine, both
// in statement and type positions.
//
// PURPOSE: pins that the composite-type projection fallback in process_local
// does not over-reject: a correctly-directed `sub := append(chs, ch)` then
// `<-sub[0]` must stay ACCEPT end to end.
//
// EXPECTED-VERDICT: ACCEPT
// ============================================================================
package main

fn ok(chs []chan int, ch chan int) {
    sub := append(chs, ch)
    var a [<-sub[0]]int
    _ = a
}
