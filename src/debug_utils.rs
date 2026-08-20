pub fn trace<T>(name: impl Into<String>, f: impl FnOnce() -> T) -> T {
    cosmic::iced::debug::time_with(name, f)
}