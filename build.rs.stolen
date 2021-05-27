// TODO: static linking? (undefined symbol)
fn main() {
    // #[cfg(feature="static")]
    println!("cargo:rustc-link-lib=bass");
    // #[cfg(feature="static")]
    println!("cargo:rustc-link-lib=bass_fx");
    println!("cargo:rerun-if-changed=src/bass/wrapper.h");
    bindgen::Builder::default()
        .header("src/bass/wrapper.h")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings")
        .write_to_file("src/bass/bindings.rs")
        .expect("Couldn't write bindings!");
}
