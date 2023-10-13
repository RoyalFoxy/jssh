use std::io::{self, Write};

use lazy_static::lazy_static;

macro_rules! lazy_statics {
    ($($name:ident : $value:expr),*) => {
        lazy_static! {
            $(
                pub static ref $name: String = $value.to_string();
            )*
        }

        $(
            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.as_str())
                }
            }
        )*
    };
}

lazy_statics! {
    NEWLINE: "\n",
    RETURN: "\r",
    NEWLINE_RETURN: "\n\r"
}

pub fn flush() {
    io::stdout().flush().unwrap();
}
