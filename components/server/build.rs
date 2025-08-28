use std::fs;
use std::path::Path;
use std::env;

fn main() {
    // Cargo sets CARGO_FEATURE_<name> environment variables for active features
    let has_full = env::var("CARGO_FEATURE_FULL").is_ok();
    let has_standard = env::var("CARGO_FEATURE_STANDARD").is_ok();
    let has_basic = env::var("CARGO_FEATURE_BASIC").is_ok();
    let has_tools = env::var("CARGO_FEATURE_TOOLS").is_ok();
    let has_resources = env::var("CARGO_FEATURE_RESOURCES").is_ok();
    let has_prompts = env::var("CARGO_FEATURE_PROMPTS").is_ok();
    
    // Determine which WIT file to use based on enabled features
    let wit_source = if has_full {
        "wit-variants/server-full.wit"
    } else if has_standard {
        "wit-variants/server-standard.wit"
    } else if has_basic || (has_tools && has_resources) {
        "wit-variants/server-basic.wit"
    } else if has_prompts {
        "wit-variants/server-prompts.wit"
    } else if has_resources {
        "wit-variants/server-resources.wit"
    } else if has_tools {
        "wit-variants/server-tools.wit"
    } else {
        // Default to tools-only if nothing specified
        "wit-variants/server-tools.wit"
    };
    
    // Copy the selected WIT file to world.wit
    let source_path = Path::new(wit_source);
    let dest_path = Path::new("wit/world.wit");
    
    // Read the source file
    let content = fs::read_to_string(source_path)
        .expect(&format!("Failed to read {}", wit_source));
    
    // Write to world.wit
    fs::write(dest_path, content)
        .expect("Failed to write wit/world.wit");
    
    println!("cargo:warning=Using WIT file: {}", wit_source);
    
    // Tell Cargo to rerun this build script if any WIT files change
    println!("cargo:rerun-if-changed=wit-variants/server-tools.wit");
    println!("cargo:rerun-if-changed=wit-variants/server-resources.wit");
    println!("cargo:rerun-if-changed=wit-variants/server-prompts.wit");
    println!("cargo:rerun-if-changed=wit-variants/server-basic.wit");
    println!("cargo:rerun-if-changed=wit-variants/server-standard.wit");
    println!("cargo:rerun-if-changed=wit-variants/server-full.wit");
    println!("cargo:rerun-if-changed=Cargo.toml");
}