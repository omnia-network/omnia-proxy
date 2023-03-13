use std::env;

pub fn get_env_var(var_name: &str) -> String {
    match env::var(var_name) {
        Ok(val) => val,
        Err(e) => panic!("Error getting env var: {}", e),
    }
}
