use std::io::{stdout, IsTerminal};

pub fn is_color_enabled() -> bool {
    if let Ok(force_color) = std::env::var("FORCE_COLOR") {
        if is_env_affirmative(&force_color) {
            return true;
        }
    };
    if let Ok(no_color) = std::env::var("NO_COLOR") {
        if is_env_affirmative(&no_color) {
            return false;
        }
    };

    is_terminal()
}

pub fn is_env_negative(value: &str) -> bool {
    let value = value.to_lowercase();
    value == "false" || value == "no" || value == "0"
}

pub fn is_env_affirmative(value: &str) -> bool {
    !is_env_negative(value)
}

pub fn is_terminal() -> bool {
    stdout().is_terminal()
}
