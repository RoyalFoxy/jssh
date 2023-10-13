use lazy_static::lazy_static;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

lazy_static! {
    pub static ref CONFIG: Mutex<Config> =
        Mutex::new(confy::load::<Config>(env!("CARGO_PKG_NAME"), Some("config")).unwrap());
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    pub start_up_file: String,
    pub history_file: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            start_up_file: String::from("~/.jssh.js"),
            history_file: String::from("~/.jssh_history"),
        }
    }
}
