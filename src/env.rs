use std::env;

use dotenvy::dotenv;

pub fn get_env_var(var_name: &str) -> String {
    match env::var(var_name) {
        Ok(val) => val,
        Err(e) => panic!("Error getting env var: {} {}", var_name, e),
    }
}

pub fn load_env_variables() -> Result<std::path::PathBuf, dotenvy::Error> {
    dotenv()
}
