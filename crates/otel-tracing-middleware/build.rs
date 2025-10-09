use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};
use wac_graph::{CompositionGraph, EncodeOptions};

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .context("Failed to find workspace root")?;
    let target_dir = workspace_root.join("target/wasm32-wasip2/release");

    let providers_path = target_dir.join("otel_providers.wasm");
    let context_provider_path = target_dir.join("context_provider.wasm");
    let otel_transport_path = target_dir.join("otel_transport.wasm");
    let otel_trace_path = target_dir.join("otel_trace.wasm");
    let middleware_path = target_dir.join("otel_tracing_middleware.wasm");

    // Add rerun-if-changed for all input components
    println!("cargo:rerun-if-changed={}", providers_path.display());
    println!("cargo:rerun-if-changed={}", context_provider_path.display());
    println!("cargo:rerun-if-changed={}", otel_transport_path.display());
    println!("cargo:rerun-if-changed={}", otel_trace_path.display());
    println!("cargo:rerun-if-changed={}", middleware_path.display());

    if !providers_path.exists()
        || !context_provider_path.exists()
        || !otel_transport_path.exists()
        || !otel_trace_path.exists()
        || !middleware_path.exists()
    {
        println!("cargo:warning=One or more OTEL components not found. Skipping pre-composition.");
        println!("cargo:warning=Run: cargo build --workspace --target wasm32-wasip2 --release");
        return Ok(());
    }

    println!("cargo:warning=Pre-composing OTEL middleware with SDK dependencies...");

    let composed_bytes = build_composition(
        &providers_path,
        &context_provider_path,
        &otel_transport_path,
        &otel_trace_path,
        &middleware_path,
    )?;

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let output_path = out_dir.join("otel-middleware-composed.wasm");
    std::fs::write(&output_path, &composed_bytes)
        .context("Failed to write composed middleware")?;

    println!(
        "cargo:warning=✅ Pre-composed OTEL middleware: {:?}",
        output_path
    );
    println!(
        "cargo:warning=   Size: {} KB",
        composed_bytes.len() / 1024
    );

    Ok(())
}

/// Pre-compose OTEL middleware with all SDK dependencies
///
/// Input: middleware + providers + SDK components (context, transport, trace)
/// Output: Single middleware.wasm with only MCP-related imports/exports:
///   - import: wasmcp:mcp/incoming-handler (to forward requests)
///   - import: wasmcp:otel/otel-config (to get provider config from handler)
///   - export: wasmcp:mcp/incoming-handler (middleware entry point)
///   - export: wasmcp:otel/trace-instrumentation (for handler custom spans)
fn build_composition(
    providers_path: &Path,
    context_provider_path: &Path,
    otel_transport_path: &Path,
    otel_trace_path: &Path,
    middleware_path: &Path,
) -> Result<Vec<u8>> {
    let mut graph = CompositionGraph::new();

    let load_package = |graph: &mut CompositionGraph,
                        name: &str,
                        path: &Path|
     -> Result<wac_graph::types::Package> {
        wac_graph::types::Package::from_file(
            &format!("wasmcp:{}", name),
            None,
            path,
            graph.types_mut(),
        )
        .with_context(|| format!("Failed to load package '{}' from {}", name, path.display()))
    };

    println!("   Loading packages...");

    let providers_pkg = load_package(&mut graph, "otel-providers", providers_path)?;
    let context_provider_pkg = load_package(&mut graph, "context-provider", context_provider_path)?;
    let otel_transport_pkg = load_package(&mut graph, "otel-transport", otel_transport_path)?;
    let otel_trace_pkg = load_package(&mut graph, "otel-trace", otel_trace_path)?;
    let middleware_pkg = load_package(&mut graph, "otel-tracing-middleware", middleware_path)?;

    println!("   Registering packages...");

    let providers_id = graph.register_package(providers_pkg)?;
    let context_provider_id = graph.register_package(context_provider_pkg)?;
    let otel_transport_id = graph.register_package(otel_transport_pkg)?;
    let otel_trace_id = graph.register_package(otel_trace_pkg)?;
    let middleware_id = graph.register_package(middleware_pkg)?;

    println!("   Building composition graph...");

    // Step 1: Instantiate providers and wire its SDK dependencies
    let providers_inst = graph.instantiate(providers_id);

    // Note: providers needs transport to be wired BEFORE we can use it
    // We'll wire it after we create the transport instances below

    let common_providers_export = graph.alias_instance_export(
        providers_inst,
        "wasi:otel-providers/common-providers@0.1.0",
    )?;

    // Step 2: Instantiate SDK components and wire them together
    let context_provider_inst = graph.instantiate(context_provider_id);
    let context_export = graph.alias_instance_export(
        context_provider_inst,
        "wasi:otel-sdk/context@0.1.0-alpha.3",
    )?;

    let otel_transport_inst = graph.instantiate(otel_transport_id);
    let transport_export = graph.alias_instance_export(
        otel_transport_inst,
        "wasi:otel-sdk/transport@0.1.0-alpha.3",
    )?;

    let otel_trace_inst = graph.instantiate(otel_trace_id);
    graph.set_instantiation_argument(
        otel_trace_inst,
        "wasi:otel-sdk/context@0.1.0-alpha.3",
        context_export,
    )?;
    graph.set_instantiation_argument(
        otel_trace_inst,
        "wasi:otel-sdk/transport@0.1.0-alpha.3",
        transport_export,
    )?;

    let trace_export = graph.alias_instance_export(
        otel_trace_inst,
        "wasi:otel-sdk/trace@0.1.0-alpha.3",
    )?;

    // http-transport comes from otel-transport, not otel-trace
    let http_transport_export = graph.alias_instance_export(
        otel_transport_inst,
        "wasi:otel-sdk/http-transport@0.1.0-alpha.3",
    )?;

    // Step 2b: Wire providers' SDK imports now that we have the exports
    println!("   Wiring providers dependencies...");
    graph.set_instantiation_argument(
        providers_inst,
        "wasi:otel-sdk/transport@0.1.0-alpha.3",
        transport_export,
    )?;
    graph.set_instantiation_argument(
        providers_inst,
        "wasi:otel-sdk/http-transport@0.1.0-alpha.3",
        http_transport_export,
    )?;

    // Step 3: Instantiate middleware and wire all OTEL SDK dependencies
    let middleware_inst = graph.instantiate(middleware_id);

    println!("   Wiring middleware dependencies...");
    // Note: wasi:otel-sdk/common is just types, not wired as a component instance
    println!("   Wiring context...");
    graph.set_instantiation_argument(
        middleware_inst,
        "wasi:otel-sdk/context@0.1.0-alpha.3",
        context_export,
    )?;
    println!("   Wiring trace...");
    graph.set_instantiation_argument(
        middleware_inst,
        "wasi:otel-sdk/trace@0.1.0-alpha.3",
        trace_export,
    )?;
    println!("   Wiring transport...");
    graph.set_instantiation_argument(
        middleware_inst,
        "wasi:otel-sdk/transport@0.1.0-alpha.3",
        transport_export,
    )?;
    println!("   Wiring http-transport...");
    graph.set_instantiation_argument(
        middleware_inst,
        "wasi:otel-sdk/http-transport@0.1.0-alpha.3",
        http_transport_export,
    )?;
    println!("   Wiring common-providers...");
    graph.set_instantiation_argument(
        middleware_inst,
        "wasi:otel-providers/common-providers@0.1.0",
        common_providers_export,
    )?;
    println!("   All middleware dependencies wired successfully");

    // Step 4: Export the middleware's MCP and OTEL interfaces
    // These will be wired by wasmcp compose at final composition time
    let incoming_handler_export = graph.alias_instance_export(
        middleware_inst,
        "wasmcp:mcp/incoming-handler@0.3.0",
    )?;
    graph.export(
        incoming_handler_export,
        "wasmcp:mcp/incoming-handler@0.3.0",
    )?;

    let trace_instrumentation_export = graph.alias_instance_export(
        middleware_inst,
        "wasmcp:otel/trace-instrumentation@0.3.0",
    )?;
    graph.export(
        trace_instrumentation_export,
        "wasmcp:otel/trace-instrumentation@0.3.0",
    )?;

    println!("   Encoding component...");
    let bytes = graph.encode(EncodeOptions::default())?;

    Ok(bytes)
}
