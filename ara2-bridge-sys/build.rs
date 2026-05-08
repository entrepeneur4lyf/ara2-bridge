use std::env;
use std::fs;
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

    let bindings_path = out.join("ara2_bindings.rs");
    bindings
        .write_to_file(&bindings_path)
        .expect("write failed");

    let vtables = [
        "ARAAudioAccessControllerInterface",
        "ARAArchivingControllerInterface",
        "ARAModelUpdateControllerInterface",
        "ARAPlaybackControllerInterface",
        "ARAContentAccessControllerInterface",
    ];

    let mut gen = String::from("// Auto-generated host vtable constructors.\n\n");

    for vname in &vtables {
        let fn_name = vname.replace("Interface", "_vtable").to_lowercase();
        gen.push_str(&format!(
            "#[allow(non_snake_case)]\n\
             pub fn build_{fn_name}() -> {vname} {{\n\
             \x20   let mut v: {vname} = unsafe {{ std::mem::zeroed() }};\n\
             \x20   v.structSize = std::mem::size_of::<{vname}>() as ARASize;\n\
             \x20   v\n\
             }}\n\n"
        ));
    }

    fs::write(out.join("host_vtables.rs"), gen).unwrap();
    println!("cargo:rerun-if-changed=ARAInterface.h");
}
