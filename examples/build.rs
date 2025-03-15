fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .compile_protos(&["dict_example.proto"], &["."])?;
    Ok(())
}
