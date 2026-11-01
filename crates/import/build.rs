use sha2::{Digest, Sha256};
fn main() {
    let mut hash = Sha256::new();
    for path in [
        "Cargo.toml",
        "build.rs",
        "src/lib.rs",
        "src/iso.rs",
        "src/main.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
        hash.update(path.as_bytes());
        hash.update(std::fs::read(path).expect("import build input"));
    }
    println!(
        "cargo:rustc-env=RR_IMPORT_BUILD_SHA256={:x}",
        hash.finalize()
    );
}
