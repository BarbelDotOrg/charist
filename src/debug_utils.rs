use std::time::Instant;

pub fn trace<T>(name: impl Into<String> + std::fmt::Display, f: impl FnOnce() -> T) -> T {
    let now = Instant::now();
    let result = f();
    let end = now.elapsed();
    println!("{} {:?}", name, end);
    result
}
