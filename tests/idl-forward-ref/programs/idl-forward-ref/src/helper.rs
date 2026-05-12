// Precise-capture syntax stabilised in Rust 1.82 (RFC 3617).
// rustc 1.82+ accepts this; syn v1.0.109 (used by anchor-syn's
// CrateContext::parse) cannot parse `+ use<T>` and returns
// "expected identifier" — matching the error in issue #4513.

pub fn identity<T: Clone>(x: T) -> impl Clone + use<T> {
    x
}
