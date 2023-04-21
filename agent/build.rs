fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .out_dir("lib/src/api/")
        .compile(&["../shared/proto/lambdo.proto"], &["../shared/proto"])?;
    Ok(())
}
