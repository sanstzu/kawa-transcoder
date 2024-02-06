fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::compile_protos("proto/transcoder.proto")?;

    // panics if not linux
    if std::env::consts::OS != "linux" {
        panic!("This program only runs on linux");
    }
    Ok(())
}
