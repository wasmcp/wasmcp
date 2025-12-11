use anyhow::Result;

use super::github::get_published_component_wit_deps;
use super::version::Version;

pub fn display_component_versions(
    component: &str,
    cargo_version: &Version,
    wit_version: Option<&Version>,
    latest_release: Option<&Version>,
    local_mcp_wit: Option<&str>,
) -> Result<()> {
    println!("  Cargo.toml:     {}", cargo_version);

    if let Some(wit_ver) = wit_version {
        println!("  world.wit:      {}", wit_ver);
    }

    if let Some(local_mcp) = local_mcp_wit {
        println!("  Local MCP WIT:  {}", local_mcp);
    }

    if let Some(release_ver) = latest_release {
        println!("  GitHub release: {}", release_ver);

        // Check what MCP WIT version the published component uses
        if let Ok(Some(published_mcp_wit)) = get_published_component_wit_deps(component, release_ver) {
            display_mcp_wit_comparison(&published_mcp_wit, local_mcp_wit)?;
        }
    } else {
        println!("  GitHub release: (none)");
    }

    Ok(())
}

fn display_mcp_wit_comparison(
    published_mcp_wit: &str,
    local_mcp_wit: Option<&str>,
) -> Result<()> {
    if let Some(local_mcp) = local_mcp_wit {
        if published_mcp_wit == local_mcp {
            println!("    └─ MCP WIT:   {} \x1b[32m✓\x1b[0m", published_mcp_wit);
        } else {
            println!(
                "    └─ MCP WIT:   {} \x1b[33m(local: {})\x1b[0m",
                published_mcp_wit, local_mcp
            );
        }
    } else {
        println!("    └─ MCP WIT:   {}", published_mcp_wit);
    }
    Ok(())
}

pub fn display_version_match_status(
    cargo_version: &Version,
    wit_version: Option<&Version>,
) {
    if let Some(wit_ver) = wit_version {
        if cargo_version == wit_ver {
            println!("\x1b[32m  ✓ Versions match\x1b[0m");
        } else {
            println!("\x1b[31m  ✗ Version mismatch!\x1b[0m");
            println!(
                "\x1b[31m    Cargo.toml has {} but world.wit has {}\x1b[0m",
                cargo_version, wit_ver
            );
        }
    }
}
