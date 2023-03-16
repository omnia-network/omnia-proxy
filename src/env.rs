use std::env;

use dotenvy::dotenv;

pub fn get_env_var(var_name: &str) -> String {
    match env::var(var_name) {
        Ok(val) => val,
        Err(e) => panic!("Error getting env var: {} {}", var_name, e),
    }
}

pub fn load_env_variables() -> Result<(), dotenvy::Error> {
    match env::var("ENV") {
        Ok(val) => {
            if val == "production" {
                println!("ENV set to production, skipping loading .env file");
                Ok(())
            } else {
                println!("ENV not set to production, loading .env file");
                match dotenv() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(e),
                }
            }
        }
        Err(_) => {
            println!("ENV not set, loading .env file");
            match dotenv() {
                Ok(_) => Ok(()),
                Err(e) => Err(e),
            }
        }
    }
}
