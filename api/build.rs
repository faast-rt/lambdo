fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src/")
        .message_attribute(".", "#[derive(serde::Deserialize, serde::Serialize)]")
        .type_attribute(
            ".grpc_definitions.RegisterResponse.response",
            "#[derive(serde::Deserialize, serde::Serialize)] #[serde(rename_all = \"snake_case\")]",
        )
        .compile(&["../shared/proto/lambdo.proto"], &["../shared/proto"])?;
    Ok(())
}
