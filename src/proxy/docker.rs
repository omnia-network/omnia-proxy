use std::process::Command;

use crate::{env::get_env_var, models::GenericError};

/// Runs a Wireguard [wg command](https://manpages.debian.org/unstable/wireguard-tools/wg.8.en.html)
/// through docker command on the `wireguard` container
pub fn wg_docker_command(args: Vec<&str>, use_wg_quick: bool) -> Result<String, GenericError> {
    let wireguard_container_name = get_env_var("WIREGUARD_CONTAINER_NAME");

    let output = Command::new("docker")
        .arg("exec")
        .arg(wireguard_container_name)
        .arg(if use_wg_quick { "wg-quick" } else { "wg" })
        .args(args)
        .output()
        .expect("failed to execute process");

    assert_eq!(output.status.code(), Some(0));

    match output.status.code() {
        Some(0) => {
            let res = String::from_utf8_lossy(&output.stdout);

            println!("docker stdout:\n{res}");

            Ok(res.to_string())
        }
        _ => {
            let err = format!(
                "status: {}, stderr:\n{}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            );

            println!("Error executing docker command: {err}");

            Err(err)
        }
    }
}
