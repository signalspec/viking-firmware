use std::env;

fn main() {
    assert_eq!(env::var("TARGET").unwrap(), "thumbv6m-none-eabi");

    let pkg_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("cargo::rustc-link-search={pkg_dir}");
    println!("cargo::rerun-if-changed={pkg_dir}/memory.x");
}
