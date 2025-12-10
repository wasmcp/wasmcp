fn main() {
    // Tell Cargo to rerun this build script if routing.toml changes
    println!("cargo:rerun-if-changed=routing.toml");
}
