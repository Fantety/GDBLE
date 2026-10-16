use sha2::{Digest, Sha256};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            if !matches!(
                path.file_name().and_then(|s| s.to_str()),
                Some("build" | ".gradle")
            ) {
                collect(&path, files);
            }
        } else if path
            .file_name()
            .is_some_and(|name| name != "local.properties")
            && path.extension().is_some_and(|ext| {
                matches!(
                    ext.to_str(),
                    Some("rs" | "kt" | "kts" | "xml" | "pro" | "properties")
                )
            })
        {
            files.push(path);
        }
    }
}

fn output(command: &str, arguments: &[&str]) -> String {
    Command::new(command)
        .args(arguments)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn main() {
    let root = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let mut files: Vec<PathBuf> = [
        "Cargo.toml",
        "Cargo.lock",
        "build.rs",
        "addons/gdble/gdble.gdextension",
        "addons/gdble/gdble.gdap",
        "addons/gdble/permission_startup.gd",
    ]
    .iter()
    .map(|p| root.join(p))
    .collect();
    collect(&root.join("src"), &mut files);
    collect(&root.join("android-plugin"), &mut files);
    files.sort_by_key(|p| {
        p.strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/")
    });
    let mut digest = Sha256::new();
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        let name = path
            .strip_prefix(&root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        let content = fs::read_to_string(path)
            .expect("UTF-8 source")
            .replace("\r\n", "\n");
        digest.update((name.len() as u64).to_be_bytes());
        digest.update(name.as_bytes());
        digest.update((content.len() as u64).to_be_bytes());
        digest.update(content.as_bytes());
    }
    // Rerun on new source files and branch changes as well as edits.
    println!("cargo:rerun-if-changed=src");
    println!("cargo:rerun-if-changed=android-plugin/plugin/src");
    println!("cargo:rerun-if-changed=.git/HEAD");
    println!("cargo:rerun-if-changed=.git/refs");
    let digest = format!("{:x}", digest.finalize());
    let version = env::var("CARGO_PKG_VERSION").unwrap();
    let commit = output("git", &["rev-parse", "HEAD"]);
    let toolchain = output(&env::var("RUSTC").unwrap(), &["--version"]);
    let target = env::var("TARGET").unwrap();
    let test_harness = env::var_os("CARGO_FEATURE_TEST_HARNESS").is_some();
    let info = format!("GDBLE_BUILD_INFO_V1:{{\"version\":{version:?},\"source_commit\":{commit:?},\"source_sha256\":{digest:?},\"toolchain\":{toolchain:?},\"target\":{target:?},\"test_harness\":{test_harness}}}\0");
    let source = format!(
        "#[used]\n#[no_mangle]\npub static GDBLE_BUILD_INFO: [u8; {}] = {:?};\n",
        info.len(),
        info.as_bytes()
    );
    fs::write(
        PathBuf::from(env::var("OUT_DIR").unwrap()).join("build_info.rs"),
        source,
    )
    .unwrap();
}
