use std::{env, fs, path::PathBuf};

fn main() {
    #[cfg(windows)]
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        println!("cargo:rerun-if-changed=packaging/windows/icon.ico");
        winresource::WindowsResource::new()
            .set_icon("packaging/windows/icon.ico")
            .set("ProductName", "Tempest 2032")
            .set("FileDescription", "Tempest 2032")
            .compile()
            .expect("could not embed Windows application resources");
    }
    let directory = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("levels");
    println!("cargo:rerun-if-changed={}", directory.display());
    let mut paths: Vec<_> = fs::read_dir(&directory)
        .expect("could not read bundled levels")
        .map(|entry| entry.expect("could not read bundled level entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "toml")
        })
        .collect();
    paths.sort();
    assert!(!paths.is_empty(), "no bundled levels found");
    let mut source = String::from("const BUNDLED_LEVELS: &[(&str, &str)] = &[\n");
    for path in paths {
        let name = path.file_name().unwrap().to_str().unwrap();
        source.push_str(&format!("({name:?}, include_str!({path:?})),\n"));
    }
    source.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("levels.rs"),
        source,
    )
    .expect("could not generate bundled level catalog");
}
