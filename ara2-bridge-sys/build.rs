use std::env;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());

    let bindings = bindgen::Builder::default()
        .header("ARAInterface.h")
        .clang_args(&["-x", "c", "-std=c11"])
        .allowlist_type("ARA.*")
        .allowlist_var("kARA.*")
        .allowlist_function("ARA.*")
        .default_enum_style(bindgen::EnumVariation::Rust {
            non_exhaustive: false,
        })
        .size_t_is_usize(true)
        .generate()
        .expect("bindgen failed");

    bindings
        .write_to_file(out.join("ara2_bindings.rs"))
        .expect("write failed");

    println!("cargo:rerun-if-changed=ARAInterface.h");
}
