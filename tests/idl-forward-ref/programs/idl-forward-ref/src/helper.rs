// Precise-capture syntax stabilised in Rust 1.82 (RFC 3617).
// syn v1 could not parse `+ use<T>` and returned "expected identifier"
// (issue #4513); syn 2 handles it correctly.

#[allow(dead_code)]
pub fn identity<T: Clone>(x: T) -> impl Clone + use<T> {
    x
}
