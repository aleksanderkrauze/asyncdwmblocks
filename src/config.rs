#[derive(Debug, PartialEq, Clone)]
pub struct Config {
    pub button_env_variable: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            button_env_variable: String::from("BUTTON"),
        }
    }
}
