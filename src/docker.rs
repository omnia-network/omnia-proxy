use std::process::Command;

pub fn docker(args: &str) {
    let output = Command::new("docker")
        .arg(args)
        .output()
        .expect("failed to execute process");

    println!("status: {}", output.status);
    println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
}