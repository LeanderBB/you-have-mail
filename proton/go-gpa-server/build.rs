use bindgen::EnumVariation;
use std::env;
use std::io::{stderr, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

const GO_LIB_NAME: &str = "go-gpa-server";

fn main() {
    let (lib_dir, lib_path) = target_path_for_go_lib();

    println!("cargo:rustc-link-search={}", lib_dir.to_str().unwrap());
    println!("cargo:rustc-link-lib={GO_LIB_NAME}");

    build_go_lib(&lib_path);
    generate_bindings_go_for_lib(&lib_dir)
}

fn target_path_for_go_lib() -> (PathBuf, PathBuf) {
    let lib_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    (
        lib_dir.clone(),
        lib_dir.join(format!("lib{GO_LIB_NAME}.so")),
    )
}

fn build_go_lib(lib_path: &Path) {
    let mut command = Command::new("go");

    #[cfg(any(target_os = "linux", target_os = "android"))]
    command.env("CGO_LDFLAGS", "-Wl,--build-id=none");
    command.arg("build");
    command.arg("-ldflags=-buildid=");
    command.arg("-trimpath");
    command.arg("-o");
    command.arg(lib_path);

    command.arg("-buildmode=c-shared");
    command.arg("lib.go");
    println!("cargo:rerun-if-changed=go/lib.go");
    command.current_dir("go");

    let output = command.output().unwrap();
    if !output.status.success() {
        eprint!("Failed to compile go library:");
        stderr()
            .write_all(output.stderr.as_slice())
            .expect("Error write failed");
        panic!("Go lib build failed");
    }
}

fn generate_bindings_go_for_lib(lib_dir: &Path) {
    let header = lib_dir.join("libgo-gpa-server.h");

    let generated_bindings = PathBuf::from(env::var("OUT_DIR").unwrap()).join("go-gpa-server.rs");

    let bindings = bindgen::Builder::default()
        .header(header.to_str().unwrap())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .derive_debug(false)
        .impl_debug(false)
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .generate()
        .expect("Unable to generate go lib bindings");

    bindings
        .write_to_file(generated_bindings)
        .expect("Failed to write bindings to file");
}
