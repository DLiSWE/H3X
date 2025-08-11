fn main() {
    // Re-run when your proto changes
    println!("cargo:rerun-if-changed=proto/h3x.proto");
    println!("cargo:rerun-if-changed=proto");

    // Helpful logs so you can SEE the build script run
    println!("cargo:warning=prost build.rs starting");

    // Use vendored protoc so Windows PATH isn't a problem
    let protoc = protoc_bin_vendored::protoc_bin_path().expect("vendored protoc");

    // Generate directly into your source tree
    prost_build::Config::new()
        .protoc_executable(protoc)
        .out_dir("src/protocol") // <= writes src/protocol/h3x.rs
        .compile_protos(&["proto/h3x.proto"], &["proto"])
        .expect("prost build failed");

    println!("cargo:warning=prost build.rs finished");
}
