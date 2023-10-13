use std::path::PathBuf;

#[inline]
pub fn expand(path: &str) -> String {
    shellexpand::full(path).unwrap().to_string()
}

#[inline]
pub fn expand_path(path: &str) -> PathBuf {
    PathBuf::from(expand(path))
}
