use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src/vm_manager/vmm")
        .message_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .type_attribute(
            ".grpc_definitions.RegisterResponse.response",
            "#[derive(serde::Deserialize, serde::Serialize)] #[serde(rename_all = \"snake_case\")]",
        )
        .compile(&["../shared/proto/lambdo.proto"], &["../shared/proto"])?;

    let _ = Command::new(std::env::var("RUSTFMT").unwrap_or_else(|_| "rustfmt".to_owned()))
        .arg("--edition")
        .arg("2021")
        .arg("--emit")
        .arg("files")
        .arg(format!(
            "{}/{}",
            "src/vm_manager/vmm", "grpc_definitions.rs"
        ))
        .output();

    Ok(())
}
