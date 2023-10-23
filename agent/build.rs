use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("lib/src/api/")
        .compile(&["../shared/proto/lambdo.proto"], &["../shared/proto"])?;

    let _ = Command::new(std::env::var("RUSTFMT").unwrap_or_else(|_| "rustfmt".to_owned()))
        .arg("--emit")
        .arg("files")
        .arg(format!("{}/{}", "lib/src/api/", "grpc_definitions.rs"))
        .output();

    Ok(())
}
