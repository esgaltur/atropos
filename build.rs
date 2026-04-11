fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Compile the gRPC protocol buffers into Rust code
    tonic_build::compile_protos("infra/proto/atropos.proto")?;
    Ok(())
}
