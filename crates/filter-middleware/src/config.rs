use crate::bindings::exports::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::mcp_v20250618::mcp::*;
use crate::bindings::wasmcp::mcp_v20250618::server_handler as downstream;
use crate::helpers::to_downstream_ctx;
use crate::types::*;
use crate::INTERNAL_REQUEST_ID_VALUE;
use std::collections::{HashMap, HashSet};

/// Load and aggregate all routing configs from config:// resources.
///
/// Discovery process:
/// 1. Call resources/list to find config://routing-* resources
/// 2. Load each config via resources/read
/// 3. Aggregate with "Deny Trumps Allow" semantics
///
/// Returns aggregated config or error if no valid configs found.
pub fn load_and_aggregate_configs(ctx: &MessageContext) -> Result<AggregatedConfig, String> {
    // Discover all routing config URIs
    let config_uris = discover_routing_configs(ctx)?;

    // Read and parse each config
    let mut configs = Vec::new();
    let mut load_errors = Vec::new();

    for uri in config_uris.iter() {
        match read_config_from_uri(ctx, uri) {
            Ok(config) => configs.push((uri.clone(), config)),
            Err(e) => {
                // Log error but continue with other configs
                eprintln!("Warning: Failed to load config {}: {}", uri, e);
                load_errors.push((uri.clone(), e));
            }
        }
    }

    // Check if we have any valid configs
    if configs.is_empty() {
        // Build detailed error message
        let mut error_msg = String::from("No valid routing configs found. ");
        if !load_errors.is_empty() {
            error_msg.push_str(&format!("Failed to load {} config(s):\n", load_errors.len()));
            for (uri, error) in load_errors.iter() {
                error_msg.push_str(&format!("  - {}: {}\n", uri, error));
            }
        }
        error_msg.push_str(&format!("Discovered {} config URI(s) total.", config_uris.len()));
        return Err(error_msg);
    }

    // Log summary if some configs failed but others succeeded
    if !load_errors.is_empty() {
        eprintln!(
            "Config loading summary: {} succeeded, {} failed",
            configs.len(),
            load_errors.len()
        );
    }

    // Aggregate configs with "Deny Trumps Allow" logic
    Ok(aggregate_configs(configs))
}

/// Discover all config://routing-* resources with application/toml mime-type
pub fn discover_routing_configs(ctx: &MessageContext) -> Result<Vec<String>, String> {
    // Send resources/list request through downstream handler
    let list_req = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    let list_msg = ClientMessage::Request((RequestId::Number(INTERNAL_REQUEST_ID_VALUE), list_req));

    let all_resources = match downstream::handle(&to_downstream_ctx(ctx), list_msg) {
        Some(Ok(ServerResult::ResourcesList(result))) => result.resources,
        Some(Ok(_)) => return Err("Unexpected result type from resources/list".to_string()),
        Some(Err(e)) => return Err(format!("Resources list failed: {:?}", e)),
        None => return Err("resources/list not available".to_string()),
    };

    // Filter for config://routing-* URIs with application/toml mime type
    let config_uris: Vec<String> = all_resources
        .iter()
        .filter(|res| {
            // Check URI pattern
            if !res.uri.starts_with("config://routing-") && res.uri != "routing://config" {
                return false;
            }

            // Check MIME type if available
            if let Some(opts) = &res.options {
                if let Some(mime) = &opts.mime_type {
                    return mime == "application/toml";
                }
            }

            // If no MIME type specified, assume it's valid (for backward compatibility)
            true
        })
        .map(|res| res.uri.clone())
        .collect();

    if config_uris.is_empty() {
        return Err("No routing configs found (looking for config://routing-* or routing://config with application/toml)".to_string());
    }

    Ok(config_uris)
}

/// Read and parse a single config from URI
pub fn read_config_from_uri(ctx: &MessageContext, uri: &str) -> Result<RoutingConfig, String> {
    let request = ReadResourceRequest {
        uri: uri.to_string(),
    };

    let downstream_req = ClientRequest::ResourcesRead(request);
    let downstream_msg = ClientMessage::Request((RequestId::Number(INTERNAL_REQUEST_ID_VALUE), downstream_req));

    let result = match downstream::handle(&to_downstream_ctx(ctx), downstream_msg) {
        Some(Ok(ServerResult::ResourcesRead(result))) => result,
        Some(Ok(_)) => return Err(format!("Unexpected result type from resources/read for {}", uri)),
        Some(Err(e)) => return Err(format!("Resource read failed for {}: {:?}", uri, e)),
        None => return Err(format!("Resource not found: {}", uri)),
    };

    let contents = result
        .contents
        .first()
        .ok_or_else(|| format!("{} has no contents", uri))?;

    // Extract text from resource-contents variant
    let text_contents = match contents {
        ResourceContents::Text(t) => t,
        ResourceContents::Blob(_) => {
            return Err(format!("{} is binary, expected text", uri))
        }
    };

    // Extract string from text-data variant
    let config_text = match &text_contents.text {
        TextData::Text(s) => s,
        TextData::TextStream(_) => {
            return Err(format!("{} is streamed, expected inline text", uri))
        }
    };

    // Parse TOML
    toml::from_str(config_text).map_err(|e| format!("TOML parse error in {}: {}", uri, e))
}

/// Extract config sources from configs list
fn extract_config_sources(configs: &[(String, RoutingConfig)]) -> Vec<ConfigSource> {
    configs
        .iter()
        .map(|(uri, config)| ConfigSource {
            uri: uri.clone(),
            version: config.version.clone(),
        })
        .collect()
}

/// Aggregate global tag filters from all configs (union with deduplication)
fn aggregate_global_tag_filters(configs: &[(String, RoutingConfig)]) -> HashMap<String, Vec<String>> {
    let mut global_filters: HashMap<String, Vec<String>> = HashMap::new();

    // Collect all tag filters from all configs
    for (_uri, config) in configs {
        for (tag_name, tag_value) in &config.global_tag_filters {
            let values = tag_filter_value_to_vec(tag_value);
            global_filters
                .entry(tag_name.clone())
                .or_insert_with(Vec::new)
                .extend(values);
        }
    }

    // Deduplicate values for each tag
    for values in global_filters.values_mut() {
        values.sort();
        values.dedup();
    }

    global_filters
}

/// Collect all unique paths from all configs
fn collect_unique_paths(configs: &[(String, RoutingConfig)]) -> HashSet<String> {
    let mut paths = HashSet::new();
    for (_uri, config) in configs {
        for path in config.path_rules.keys() {
            paths.insert(path.clone());
        }
    }
    paths
}

/// Aggregate rules for a specific path from all configs
fn aggregate_path_rule(path: &str, configs: &[(String, RoutingConfig)]) -> AggregatedPathRule {
    let mut agg_rule = AggregatedPathRule {
        whitelist: Vec::new(),
        blacklist: Vec::new(),
        tag_filters: HashMap::new(),
        sources: RuleSources {
            whitelist_from: Vec::new(),
            blacklist_from: Vec::new(),
            tag_filters_from: Vec::new(),
        },
    };

    // Collect rules from all configs
    for (uri, config) in configs {
        if let Some(rule) = config.path_rules.get(path) {
            // Aggregate whitelist (union)
            if let Some(whitelist) = &rule.whitelist {
                agg_rule.whitelist.extend(whitelist.clone());
                agg_rule.sources.whitelist_from.push(uri.clone());
            }

            // Aggregate blacklist (union - any config can deny)
            if let Some(blacklist) = &rule.blacklist {
                agg_rule.blacklist.extend(blacklist.clone());
                agg_rule.sources.blacklist_from.push(uri.clone());
            }

            // Aggregate tag filters (union per path)
            for (tag_name, tag_value) in &rule.tag_filters {
                let values = tag_filter_value_to_vec(tag_value);
                agg_rule
                    .tag_filters
                    .entry(tag_name.clone())
                    .or_insert_with(Vec::new)
                    .extend(values);

                if !agg_rule.sources.tag_filters_from.contains(uri) {
                    agg_rule.sources.tag_filters_from.push(uri.clone());
                }
            }
        }
    }

    // Deduplicate whitelist and blacklist
    agg_rule.whitelist.sort();
    agg_rule.whitelist.dedup();
    agg_rule.blacklist.sort();
    agg_rule.blacklist.dedup();

    // Deduplicate tag filter values
    for values in agg_rule.tag_filters.values_mut() {
        values.sort();
        values.dedup();
    }

    agg_rule
}

/// Aggregate path rules from all configs
fn aggregate_path_rules(configs: &[(String, RoutingConfig)]) -> HashMap<String, AggregatedPathRule> {
    let mut path_rules = HashMap::new();
    let all_paths = collect_unique_paths(configs);

    // For each path, aggregate rules with "Deny Trumps Allow"
    for path in all_paths {
        let agg_rule = aggregate_path_rule(&path, configs);
        path_rules.insert(path, agg_rule);
    }

    path_rules
}

/// Aggregate multiple configs with "Deny Trumps Allow" logic
pub fn aggregate_configs(configs: Vec<(String, RoutingConfig)>) -> AggregatedConfig {
    AggregatedConfig {
        config_sources: extract_config_sources(&configs),
        global_tag_filters: aggregate_global_tag_filters(&configs),
        path_rules: aggregate_path_rules(&configs),
    }
}

/// Convert TagFilterValue enum to Vec<String> for uniform processing.
#[must_use]
pub fn tag_filter_value_to_vec(value: &TagFilterValue) -> Vec<String> {
    match value {
        TagFilterValue::Single(s) => vec![s.clone()],
        TagFilterValue::Multiple(v) => v.clone(),
    }
}

/// Find most specific (longest matching) path rule for given path.
///
/// Uses longest-prefix matching algorithm:
/// - "/mcp/calculator/advanced" matches "/mcp/calculator/advanced" over "/mcp/calculator"
/// - Returns None if no path rule matches
#[must_use]
pub fn find_most_specific_path_rule<'a>(
    path: &str,
    config: &'a AggregatedConfig,
) -> Option<&'a AggregatedPathRule> {
    // Find all matching path rules (path starts with rule path)
    let mut matches: Vec<(&str, &AggregatedPathRule)> = config
        .path_rules
        .iter()
        .filter(|(rule_path, _)| path.starts_with(rule_path.as_str()))
        .map(|(k, v)| (k.as_str(), v))
        .collect();

    // Sort by path length (descending) to get most specific first
    matches.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

    // Return most specific match
    matches.first().map(|(_, rule)| *rule)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_rule(whitelist: Vec<String>, blacklist: Vec<String>) -> AggregatedPathRule {
        AggregatedPathRule {
            whitelist,
            blacklist,
            tag_filters: HashMap::new(),
            sources: RuleSources {
                whitelist_from: vec![],
                blacklist_from: vec![],
                tag_filters_from: vec![],
            },
        }
    }

    #[test]
    fn test_tag_filter_value_to_vec_single() {
        let value = TagFilterValue::Single("math".to_string());
        let result = tag_filter_value_to_vec(&value);
        assert_eq!(result, vec!["math".to_string()]);
    }

    #[test]
    fn test_tag_filter_value_to_vec_multiple() {
        let value = TagFilterValue::Multiple(vec!["math".to_string(), "science".to_string()]);
        let result = tag_filter_value_to_vec(&value);
        assert_eq!(result, vec!["math".to_string(), "science".to_string()]);
    }

    #[test]
    fn test_find_most_specific_path_rule_exact_match() {
        let mut config = AggregatedConfig {
            path_rules: HashMap::new(),
            global_tag_filters: HashMap::new(),
            config_sources: vec![],
        };

        config.path_rules.insert(
            "/mcp".to_string(),
            create_test_rule(vec!["tool1".to_string()], vec![]),
        );

        let result = find_most_specific_path_rule("/mcp", &config);
        assert!(result.is_some());
        assert_eq!(result.unwrap().whitelist, vec!["tool1".to_string()]);
    }

    #[test]
    fn test_find_most_specific_path_rule_longest_prefix_wins() {
        let mut config = AggregatedConfig {
            path_rules: HashMap::new(),
            global_tag_filters: HashMap::new(),
            config_sources: vec![],
        };

        config.path_rules.insert(
            "/mcp".to_string(),
            create_test_rule(vec!["tool1".to_string()], vec![]),
        );
        config.path_rules.insert(
            "/mcp/calculator".to_string(),
            create_test_rule(vec!["tool2".to_string()], vec![]),
        );
        config.path_rules.insert(
            "/mcp/calculator/advanced".to_string(),
            create_test_rule(vec!["tool3".to_string()], vec![]),
        );

        // Should match longest prefix
        let result = find_most_specific_path_rule("/mcp/calculator/advanced/multiply", &config);
        assert!(result.is_some());
        assert_eq!(result.unwrap().whitelist, vec!["tool3".to_string()]);

        // Should match middle rule
        let result = find_most_specific_path_rule("/mcp/calculator/basic", &config);
        assert!(result.is_some());
        assert_eq!(result.unwrap().whitelist, vec!["tool2".to_string()]);

        // Should match shortest rule
        let result = find_most_specific_path_rule("/mcp/weather", &config);
        assert!(result.is_some());
        assert_eq!(result.unwrap().whitelist, vec!["tool1".to_string()]);
    }

    #[test]
    fn test_find_most_specific_path_rule_no_match() {
        let mut config = AggregatedConfig {
            path_rules: HashMap::new(),
            global_tag_filters: HashMap::new(),
            config_sources: vec![],
        };

        config.path_rules.insert(
            "/mcp".to_string(),
            create_test_rule(vec!["tool1".to_string()], vec![]),
        );

        let result = find_most_specific_path_rule("/api/tools", &config);
        assert!(result.is_none());
    }

    #[test]
    fn test_aggregate_configs_whitelist_union() {
        let config1 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: Some(vec!["tool1".to_string(), "tool2".to_string()]),
                        blacklist: None,
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let config2 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: Some(vec!["tool2".to_string(), "tool3".to_string()]),
                        blacklist: None,
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let configs = vec![
            ("config1".to_string(), config1),
            ("config2".to_string(), config2),
        ];

        let aggregated = aggregate_configs(configs);
        let rule = aggregated.path_rules.get("/mcp").unwrap();

        // Should be union of both whitelists (deduplicated and sorted)
        assert_eq!(rule.whitelist, vec!["tool1", "tool2", "tool3"]);
    }

    #[test]
    fn test_aggregate_configs_blacklist_union() {
        let config1 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: None,
                        blacklist: Some(vec!["bad1".to_string()]),
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let config2 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: None,
                        blacklist: Some(vec!["bad2".to_string()]),
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let configs = vec![
            ("config1".to_string(), config1),
            ("config2".to_string(), config2),
        ];

        let aggregated = aggregate_configs(configs);
        let rule = aggregated.path_rules.get("/mcp").unwrap();

        // Should be union of both blacklists
        assert_eq!(rule.blacklist, vec!["bad1", "bad2"]);
    }

    #[test]
    fn test_aggregate_configs_deny_trumps_allow_semantic() {
        // This tests that blacklists from ANY config apply
        // (the actual "deny trumps allow" enforcement happens in filtering logic)
        let config1 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: Some(vec!["tool1".to_string()]),
                        blacklist: None,
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let config2 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: None,
                        blacklist: Some(vec!["tool1".to_string()]), // Blacklists the whitelisted tool
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let configs = vec![
            ("config1".to_string(), config1),
            ("config2".to_string(), config2),
        ];

        let aggregated = aggregate_configs(configs);
        let rule = aggregated.path_rules.get("/mcp").unwrap();

        // Both lists should contain tool1
        assert!(rule.whitelist.contains(&"tool1".to_string()));
        assert!(rule.blacklist.contains(&"tool1".to_string()));
        // Filtering logic will deny this tool (tested separately)
    }

    #[test]
    fn test_aggregate_configs_global_tag_filters() {
        let config1 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: HashMap::new(),
            global_tag_filters: {
                let mut map = HashMap::new();
                map.insert("category".to_string(), TagFilterValue::Single("math".to_string()));
                map
            },
        };

        let config2 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: HashMap::new(),
            global_tag_filters: {
                let mut map = HashMap::new();
                map.insert("category".to_string(), TagFilterValue::Multiple(vec!["science".to_string()]));
                map
            },
        };

        let configs = vec![
            ("config1".to_string(), config1),
            ("config2".to_string(), config2),
        ];

        let aggregated = aggregate_configs(configs);

        // Should be union of values for same tag
        let category_values = aggregated.global_tag_filters.get("category").unwrap();
        assert_eq!(category_values.len(), 2);
        assert!(category_values.contains(&"math".to_string()));
        assert!(category_values.contains(&"science".to_string()));
    }

    #[test]
    fn test_aggregate_configs_source_tracking() {
        let config1 = RoutingConfig {
            version: "1.0".to_string(),
            path_rules: {
                let mut map = HashMap::new();
                map.insert(
                    "/mcp".to_string(),
                    PathRule {
                        whitelist: Some(vec!["tool1".to_string()]),
                        blacklist: None,
                        tag_filters: HashMap::new(),
                    },
                );
                map
            },
            global_tag_filters: HashMap::new(),
        };

        let configs = vec![
            ("config://routing-primary".to_string(), config1),
        ];

        let aggregated = aggregate_configs(configs);

        // Check config sources tracked
        assert_eq!(aggregated.config_sources.len(), 1);
        assert_eq!(aggregated.config_sources[0].uri, "config://routing-primary");

        // Check rule sources tracked
        let rule = aggregated.path_rules.get("/mcp").unwrap();
        assert_eq!(rule.sources.whitelist_from, vec!["config://routing-primary"]);
    }
}