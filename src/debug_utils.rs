use std::fmt::Display;
use std::time::Instant;

pub fn trace<T>(name: impl Display, f: impl FnOnce() -> T) -> T {
    let now = Instant::now();
    let result = f();
    let end = now.elapsed();
    println!("{} {:?}", name, end);
    result
}
