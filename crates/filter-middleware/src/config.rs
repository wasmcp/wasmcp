use crate::bindings::exports::wasmcp::mcp_v20250618::server_handler::MessageContext;
use crate::bindings::wasmcp::mcp_v20250618::mcp::*;
use crate::bindings::wasmcp::mcp_v20250618::server_handler as downstream;
use crate::helpers::to_downstream_ctx;
use crate::types::*;
use std::collections::{HashMap, HashSet};

/// Load and aggregate all routing configs
pub fn load_and_aggregate_configs(ctx: &MessageContext) -> Result<AggregatedConfig, String> {
    // Discover all routing config URIs
    let config_uris = discover_routing_configs(ctx)?;

    // Read and parse each config
    let mut configs = Vec::new();
    for uri in config_uris {
        match read_config_from_uri(ctx, &uri) {
            Ok(config) => configs.push((uri, config)),
            Err(e) => {
                // Log error but continue with other configs
                eprintln!("Warning: Failed to load config {}: {}", uri, e);
            }
        }
    }

    if configs.is_empty() {
        return Err("No valid routing configs found".to_string());
    }

    // Aggregate configs with "Deny Trumps Allow" logic
    Ok(aggregate_configs(configs))
}

/// Discover all config://routing-* resources with application/toml mime-type
pub fn discover_routing_configs(ctx: &MessageContext) -> Result<Vec<String>, String> {
    // Send resources/list request through downstream handler
    let list_req = ClientRequest::ResourcesList(ListResourcesRequest { cursor: None });
    let list_msg = ClientMessage::Request((RequestId::Number(0), list_req));

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
    let downstream_msg = ClientMessage::Request((RequestId::Number(0), downstream_req));

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

/// Aggregate multiple configs with "Deny Trumps Allow" logic
pub fn aggregate_configs(configs: Vec<(String, RoutingConfig)>) -> AggregatedConfig {
    let mut aggregated = AggregatedConfig {
        path_rules: HashMap::new(),
        global_tag_filters: HashMap::new(),
        config_sources: Vec::new(),
    };

    // Track config sources
    for (uri, config) in &configs {
        aggregated.config_sources.push(ConfigSource {
            uri: uri.clone(),
            version: config.version.clone(),
        });
    }

    // Aggregate global tag filters (union across all configs)
    for (_uri, config) in &configs {
        for (tag_name, tag_value) in &config.global_tag_filters {
            let values = tag_filter_value_to_vec(tag_value);
            aggregated
                .global_tag_filters
                .entry(tag_name.clone())
                .or_insert_with(Vec::new)
                .extend(values);
        }
    }

    // Deduplicate global tag filter values
    for values in aggregated.global_tag_filters.values_mut() {
        values.sort();
        values.dedup();
    }

    // Aggregate path rules across all configs
    // Collect all unique paths
    let mut all_paths = HashSet::new();
    for (_uri, config) in &configs {
        for path in config.path_rules.keys() {
            all_paths.insert(path.clone());
        }
    }

    // For each path, aggregate rules with "Deny Trumps Allow"
    for path in all_paths {
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

        // Collect whitelist, blacklist, and tag filters from all configs
        for (uri, config) in &configs {
            if let Some(rule) = config.path_rules.get(&path) {
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

        aggregated.path_rules.insert(path, agg_rule);
    }

    aggregated
}

/// Convert TagFilterValue enum to Vec<String>
pub fn tag_filter_value_to_vec(value: &TagFilterValue) -> Vec<String> {
    match value {
        TagFilterValue::Single(s) => vec![s.clone()],
        TagFilterValue::Multiple(v) => v.clone(),
    }
}

/// Find the most specific (longest matching) path rule
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