//! Registry data resources
//!
//! Provides access to user's wasmcp registry configuration including
//! registered components, composition profiles, and config settings.

use rmcp::ErrorData as McpError;
use rmcp::model::{
    Annotated, Annotations, RawResource, ReadResourceResult, ResourceContents, Role,
};

/// List all registry data resources
pub fn list() -> Vec<Annotated<RawResource>> {
    vec![
        Annotated {
            raw: RawResource {
                uri: "wasmcp://registry/components".into(),
                name: "Your Component Aliases (Live)".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("Live data: your currently registered component aliases from ~/.config/wasmcp/config.toml. Answers: what components have I aliased, verify alias paths, check registered components. Format: JSON. Skip: for how to register aliases, see 'registry' docs.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.7),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://registry/profiles".into(),
                name: "Your Profiles (Live)".into(),
                mime_type: Some("application/json".into()),
                title: None,
                description: Some("Live data: your composition profiles from ~/.config/wasmcp/config.toml. Answers: what profiles exist, what components in each profile, verify profile composition. Format: JSON. Skip: for how to create profiles, see 'registry' docs.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.6),
                last_modified: None,
            }),
        },
        Annotated {
            raw: RawResource {
                uri: "wasmcp://registry/config".into(),
                name: "Your Config File (Live)".into(),
                mime_type: Some("application/toml".into()),
                title: None,
                description: Some("Live data: complete ~/.config/wasmcp/config.toml file. Answers: verify config syntax, see all settings. Format: TOML. Skip: for specific data, use 'registry/components' or 'registry/profiles'.".into()),
                size: None,
                icons: None,
            },
            annotations: Some(Annotations {
                audience: Some(vec![Role::Assistant]),
                priority: Some(0.5),
                last_modified: None,
            }),
        },
    ]
}

/// Read a registry data resource (always reads fresh from disk)
pub async fn read(resource_name: &str) -> Option<Result<ReadResourceResult, McpError>> {
    match resource_name {
        "components" => Some(read_components()),
        "profiles" => Some(read_profiles()),
        "config" => Some(read_config_toml().await),
        _ => None,
    }
}

fn read_components() -> Result<ReadResourceResult, McpError> {
    // Load fresh config from disk
    let config = crate::config::load_config()
        .map_err(|e| McpError::internal_error(format!("Failed to load config: {}", e), None))?;

    let components_json = serde_json::to_string_pretty(&config.components).map_err(|e| {
        McpError::internal_error(format!("Failed to serialize components: {}", e), None)
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::text(
            components_json,
            "wasmcp://registry/components".to_string(),
        )],
    })
}

fn read_profiles() -> Result<ReadResourceResult, McpError> {
    // Load fresh config from disk
    let config = crate::config::load_config()
        .map_err(|e| McpError::internal_error(format!("Failed to load config: {}", e), None))?;

    let profiles_json = serde_json::to_string_pretty(&config.profiles).map_err(|e| {
        McpError::internal_error(format!("Failed to serialize profiles: {}", e), None)
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::text(
            profiles_json,
            "wasmcp://registry/profiles".to_string(),
        )],
    })
}

async fn read_config_toml() -> Result<ReadResourceResult, McpError> {
    let config_path = crate::config::get_config_path()
        .map_err(|e| McpError::internal_error(format!("Failed to get config path: {}", e), None))?;

    let config_content = tokio::fs::read_to_string(&config_path).await.map_err(|e| {
        McpError::internal_error(format!("Failed to read config file: {}", e), None)
    })?;

    Ok(ReadResourceResult {
        contents: vec![ResourceContents::text(
            config_content,
            "wasmcp://registry/config".to_string(),
        )],
    })
}
