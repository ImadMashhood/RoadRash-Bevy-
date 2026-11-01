use sha2::{Digest, Sha256};
fn main() {
    let mut paths = vec![
        std::path::PathBuf::from("Cargo.toml"),
        std::path::PathBuf::from("build.rs"),
        std::path::PathBuf::from("../../Cargo.lock"),
    ];
    fn collect(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        for e in std::fs::read_dir(root).unwrap() {
            let e = e.unwrap();
            if e.file_type().unwrap().is_dir() {
                collect(&e.path(), out)
            } else {
                out.push(e.path())
            }
        }
    }
    collect(std::path::Path::new("src"), &mut paths);
    paths.sort();
    let mut hash = Sha256::new();
    for path in paths {
        println!("cargo:rerun-if-changed={}", path.display());
        hash.update(path.to_string_lossy().as_bytes());
        hash.update(std::fs::read(&path).unwrap());
    }
    println!(
        "cargo:rustc-env=RR_FORMATS_BUILD_SHA256={:x}",
        hash.finalize()
    );
}
