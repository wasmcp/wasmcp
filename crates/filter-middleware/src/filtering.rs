use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;
use crate::config::find_most_specific_path_rule;
use crate::metadata::{parse_tool_metadata, tool_is_blacklisted, tool_matches_tag_filters, tool_passes_whitelist};
use crate::types::{AggregatedConfig, AggregatedPathRule, ToolWithMetadata};
use std::borrow::Cow;

/// Optimized filtering pipeline with metadata caching and path rule pre-computation.
///
/// Avoids repeated metadata parsing and path matching by caching results.
/// Use for filtering tools/list results based on HTTP path and tool metadata.
pub struct FilteringPipeline<'a> {
    /// Reference to aggregated routing configuration
    pub config: &'a AggregatedConfig,
    /// Most specific path rule for current request (pre-computed)
    pub path_rule: Option<&'a AggregatedPathRule>,
}

impl<'a> FilteringPipeline<'a> {
    /// Create new filtering pipeline for given path.
    /// Pre-computes most specific matching path rule.
    pub fn new(config: &'a AggregatedConfig, current_path: String) -> Self {
        let path_rule = find_most_specific_path_rule(&current_path, config);
        Self {
            config,
            path_rule,
        }
    }

    /// Apply all configured filters to tool list.
    ///
    /// Filtering order:
    /// 1. Path-based whitelist (if configured)
    /// 2. Path-based blacklist (always wins - "Deny Trumps Allow")
    /// 3. Tag-based filters (AND logic - must match all tags)
    ///
    /// Returns filtered tools that pass all criteria.
    /// Uses copy-on-write to avoid cloning when no filtering is needed.
    pub fn apply_filters<'b>(&self, tools: &'b [Tool]) -> Cow<'b, [Tool]> {
        // Fast path: no filters at all - zero-copy!
        if self.path_rule.is_none() && self.config.global_tag_filters.is_empty() {
            return Cow::Borrowed(tools);
        }

        // If no path rule, only apply tag filters
        if self.path_rule.is_none() {
            return Cow::Owned(self.apply_tag_filters_only(tools));
        }

        let rule = self.path_rule.unwrap();

        // If rule has no whitelist/blacklist, only apply tag filters
        if rule.whitelist.is_empty() && rule.blacklist.is_empty() {
            return Cow::Owned(self.apply_tag_filters_only(tools));
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
        Cow::Owned(self.apply_tag_filters_cached(path_filtered))
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
        // Fast path: only global filters (no merge needed)
        if self.path_rule.map(|r| r.tag_filters.is_empty()).unwrap_or(true) {
            // Use global filters directly without cloning
            if self.config.global_tag_filters.is_empty() {
                // No filters at all - extract tools directly
                return tools_with_meta.iter().map(|twm| twm.tool.clone()).collect();
            }

            // Only global filters apply
            return tools_with_meta
                .into_iter()
                .filter(|twm| tool_matches_tag_filters(&twm.metadata, &self.config.global_tag_filters))
                .map(|twm| twm.tool.clone())
                .collect();
        }

        // Slow path: need to merge global + path-specific filters
        let mut active_filters = self.config.global_tag_filters.clone();
        if let Some(rule) = self.path_rule {
            for (key, values) in &rule.tag_filters {
                active_filters.insert(key.clone(), values.clone());
            }
        }

        // Apply merged filters
        tools_with_meta
            .into_iter()
            .filter(|twm| tool_matches_tag_filters(&twm.metadata, &active_filters))
            .map(|twm| twm.tool.clone())
            .collect()
    }
}