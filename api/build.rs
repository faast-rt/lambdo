fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("src/")
        .compile(&["../shared/proto/lambdo.proto"], &["../shared/proto"])?;
    Ok(())
}
