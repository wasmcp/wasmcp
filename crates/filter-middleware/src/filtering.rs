use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;
use crate::config::find_most_specific_path_rule;
use crate::metadata::{parse_tool_metadata, tool_is_blacklisted, tool_matches_tag_filters, tool_passes_whitelist};
use crate::types::{AggregatedConfig, AggregatedPathRule, ToolMetadata, ToolWithMetadata};
use std::collections::HashMap;

/// Optimized filtering pipeline that caches metadata and path rules
pub struct FilteringPipeline<'a> {
    pub config: &'a AggregatedConfig,
    pub current_path: String,
    pub path_rule: Option<&'a AggregatedPathRule>,
}

impl<'a> FilteringPipeline<'a> {
    pub fn new(config: &'a AggregatedConfig, current_path: String) -> Self {
        let path_rule = find_most_specific_path_rule(&current_path, config);
        Self {
            config,
            current_path,
            path_rule,
        }
    }

    /// Apply all filters with optimized metadata caching
    pub fn apply_filters(&self, tools: &[Tool]) -> Vec<Tool> {
        // If no path rule, only apply tag filters
        if self.path_rule.is_none() {
            return self.apply_tag_filters_only(tools);
        }

        let rule = self.path_rule.unwrap();

        // If rule has no whitelist/blacklist, only apply tag filters
        if rule.whitelist.is_empty() && rule.blacklist.is_empty() {
            return self.apply_tag_filters_only(tools);
        }

        // Parse metadata once for all tools
        let tools_with_meta: Vec<ToolWithMetadata> = tools
            .iter()
            .map(|t| ToolWithMetadata {
                tool: t,
                metadata: parse_tool_metadata(t),
            })
            .collect();

        // Apply path filtering with cached metadata
        let path_filtered = self.apply_path_filter_cached(&tools_with_meta, rule);

        // Apply tag filters with cached metadata
        self.apply_tag_filters_cached(path_filtered)
    }

    /// Apply only tag filters when no path rule exists
    fn apply_tag_filters_only(&self, tools: &[Tool]) -> Vec<Tool> {
        // Collect active filters (global only since no path rule)
        if self.config.global_tag_filters.is_empty() {
            // No filters at all, return original vector without cloning
            return tools.to_vec();
        }

        // Parse metadata and filter
        tools
            .iter()
            .filter(|tool| {
                let metadata = parse_tool_metadata(tool);
                tool_matches_tag_filters(&metadata, &self.config.global_tag_filters)
            })
            .cloned()
            .collect()
    }

    /// Apply path-based filtering with cached metadata
    fn apply_path_filter_cached<'b>(
        &self,
        tools_with_meta: &'b [ToolWithMetadata<'b>],
        rule: &AggregatedPathRule,
    ) -> Vec<&'b ToolWithMetadata<'b>> {
        let mut filtered: Vec<&ToolWithMetadata> = Vec::new();

        // Apply whitelist first
        if !rule.whitelist.is_empty() {
            for twm in tools_with_meta {
                if tool_passes_whitelist(twm.tool, &twm.metadata, &rule.whitelist) {
                    filtered.push(twm);
                }
            }
        } else {
            // No whitelist, all tools pass this stage
            filtered.extend(tools_with_meta.iter());
        }

        // Apply blacklist (DENY TRUMPS ALLOW)
        if !rule.blacklist.is_empty() {
            filtered.retain(|twm| !tool_is_blacklisted(twm.tool, &rule.blacklist));
        }

        filtered
    }

    /// Apply tag filters with cached metadata
    fn apply_tag_filters_cached(&self, tools_with_meta: Vec<&ToolWithMetadata>) -> Vec<Tool> {
        // Collect all active tag filters (global + path-specific)
        let mut active_filters = self.config.global_tag_filters.clone();

        // Add path-specific tag filters if they exist
        if let Some(rule) = self.path_rule {
            for (key, values) in &rule.tag_filters {
                active_filters.insert(key.clone(), values.clone());
            }
        }

        // If no filters, just extract tools
        if active_filters.is_empty() {
            return tools_with_meta.iter().map(|twm| twm.tool.clone()).collect();
        }

        // Filter and extract tools in one pass
        tools_with_meta
            .into_iter()
            .filter(|twm| tool_matches_tag_filters(&twm.metadata, &active_filters))
            .map(|twm| twm.tool.clone())
            .collect()
    }
}

/// Legacy function for backward compatibility - uses optimized pipeline internally
pub fn apply_path_filter(tools: &[Tool], path: &str, config: &AggregatedConfig) -> Vec<Tool> {
    let rule = find_most_specific_path_rule(path, config);

    // If no matching rule, allow all tools
    let rule = match rule {
        Some(r) => r,
        None => return tools.to_vec(),
    };

    // If rule exists but has no whitelist/blacklist, allow all
    if rule.whitelist.is_empty() && rule.blacklist.is_empty() {
        return tools.to_vec();
    }

    // Parse metadata once
    let tools_with_meta: Vec<ToolWithMetadata> = tools
        .iter()
        .map(|t| ToolWithMetadata {
            tool: t,
            metadata: parse_tool_metadata(t),
        })
        .collect();

    let mut filtered = Vec::new();

    // Apply whitelist
    if !rule.whitelist.is_empty() {
        for twm in &tools_with_meta {
            if tool_passes_whitelist(twm.tool, &twm.metadata, &rule.whitelist) {
                filtered.push(twm.tool.clone());
            }
        }
    } else {
        filtered = tools.to_vec();
    }

    // Apply blacklist
    if !rule.blacklist.is_empty() {
        filtered.retain(|tool| !tool_is_blacklisted(tool, &rule.blacklist));
    }

    filtered
}

/// Legacy function for backward compatibility - uses optimized pipeline internally
pub fn apply_tag_filters(tools: &[Tool], path: &str, config: &AggregatedConfig) -> Vec<Tool> {
    // Collect all active tag filters (global + path-specific)
    let mut active_filters = config.global_tag_filters.clone();

    // Add path-specific tag filters (if path rule exists)
    if let Some(rule) = find_most_specific_path_rule(path, config) {
        for (key, values) in &rule.tag_filters {
            active_filters.insert(key.clone(), values.clone());
        }
    }

    // If no filters, return all tools
    if active_filters.is_empty() {
        return tools.to_vec();
    }

    // Parse metadata once and filter
    tools
        .iter()
        .filter(|tool| {
            let metadata = parse_tool_metadata(tool);
            tool_matches_tag_filters(&metadata, &active_filters)
        })
        .cloned()
        .collect()
}