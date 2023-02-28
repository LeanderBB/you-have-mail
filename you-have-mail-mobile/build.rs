use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=src/bindings.udl");
    let out_dir = PathBuf::new()
        .join("..")
        .join("you-have-mail-android")
        .join("app")
        .join("src")
        .join("main")
        .join("java");
    uniffi::generate_scaffolding("src/bindings.udl").unwrap();
    uniffi::generate_bindings(
        "src/bindings.udl".into(),
        None,
        vec!["kotlin"],
        Some(out_dir.to_str().unwrap().into()),
        None,
        false,
    )
    .unwrap();
}
