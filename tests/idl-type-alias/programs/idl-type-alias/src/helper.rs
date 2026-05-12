#[allow(dead_code)]
pub fn identity<T: Clone>(x: T) -> impl Clone + use<T> {
    x
}
