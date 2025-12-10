use crate::bindings::wasmcp::mcp_v20250618::mcp::Tool;
use crate::config::find_most_specific_path_rule;
use crate::metadata::{
    parse_tool_metadata, tool_is_blacklisted, tool_matches_tag_filters, tool_passes_whitelist,
};
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
        Self { config, path_rule }
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

        // If rule has no whitelist/blacklist AND no path-specific tag filters, only apply global tag filters
        if rule.whitelist.is_empty() && rule.blacklist.is_empty() && rule.tag_filters.is_empty() {
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

        // Apply path filtering with cached metadata (whitelist/blacklist)
        let path_filtered = if !rule.whitelist.is_empty() || !rule.blacklist.is_empty() {
            self.apply_path_filter_cached(&tools_with_meta, rule)
        } else {
            // No whitelist/blacklist, so all tools pass to tag filtering
            tools_with_meta.iter().collect()
        };

        // Apply tag filters with cached metadata (global + path-specific)
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
            filtered.retain(|twm| !tool_is_blacklisted(twm.tool, &twm.metadata, &rule.blacklist));
        }

        filtered
    }

    /// Apply tag filters with cached metadata
    fn apply_tag_filters_cached(&self, tools_with_meta: Vec<&ToolWithMetadata>) -> Vec<Tool> {
        // Fast path: only global filters (no merge needed)
        if self
            .path_rule
            .map(|r| r.tag_filters.is_empty())
            .unwrap_or(true)
        {
            // Use global filters directly without cloning
            if self.config.global_tag_filters.is_empty() {
                // No filters at all - extract tools directly
                return tools_with_meta.iter().map(|twm| twm.tool.clone()).collect();
            }

            // Only global filters apply
            return tools_with_meta
                .into_iter()
                .filter(|twm| {
                    tool_matches_tag_filters(&twm.metadata, &self.config.global_tag_filters)
                })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::wasmcp::mcp_v20250618::mcp::{Tool, ToolOptions};
    use crate::types::{AggregatedConfig, AggregatedPathRule, ConfigSource, RuleSources};
    use std::collections::HashMap;

    fn create_test_tool(name: &str, component_id: Option<&str>, tags: Vec<(&str, &str)>) -> Tool {
        let meta = if let Some(comp_id) = component_id {
            let tags_json = if tags.is_empty() {
                String::new()
            } else {
                let tags_str: Vec<String> = tags
                    .iter()
                    .map(|(k, v)| format!(r#""{}":"{}""#, k, v))
                    .collect();
                format!(r#","tags":{{{}}}"#, tags_str.join(","))
            };
            Some(format!(r#"{{"component_id":"{}"{}}}"#, comp_id, tags_json))
        } else if !tags.is_empty() {
            let tags_str: Vec<String> = tags
                .iter()
                .map(|(k, v)| format!(r#""{}":"{}""#, k, v))
                .collect();
            Some(format!(r#"{{"tags":{{{}}}}}"#, tags_str.join(",")))
        } else {
            None
        };

        Tool {
            name: name.to_string(),
            input_schema: "{}".to_string(),
            options: meta.map(|m| ToolOptions {
                description: None,
                title: None,
                meta: Some(m),
                annotations: None,
                output_schema: None,
            }),
        }
    }

    fn create_test_config(
        path_rules: HashMap<String, AggregatedPathRule>,
        global_tag_filters: HashMap<String, Vec<String>>,
    ) -> AggregatedConfig {
        AggregatedConfig {
            path_rules,
            global_tag_filters,
            config_sources: vec![ConfigSource {
                uri: "test://config".to_string(),
                version: "1.0".to_string(),
            }],
        }
    }

    #[test]
    fn test_pipeline_empty_tool_list() {
        // Empty tool list should return empty regardless of filters
        let config = create_test_config(HashMap::new(), HashMap::new());
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let result = pipeline.apply_filters(&[]);
        assert_eq!(result.len(), 0, "Empty input should return empty output");
    }

    #[test]
    fn test_pipeline_no_filters_zero_copy() {
        // With no filters, should return borrowed slice (zero-copy)
        let config = create_test_config(HashMap::new(), HashMap::new());
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![]),
            create_test_tool("tool2", None, vec![]),
        ];

        let result = pipeline.apply_filters(&tools);
        assert!(matches!(result, std::borrow::Cow::Borrowed(_)));
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_pipeline_whitelist_filtering() {
        let mut path_rules = HashMap::new();
        path_rules.insert(
            "/mcp".to_string(),
            AggregatedPathRule {
                whitelist: vec!["tool1".to_string(), "comp2".to_string()],
                blacklist: vec![],
                tag_filters: HashMap::new(),
                sources: RuleSources {
                    whitelist_from: vec![],
                    blacklist_from: vec![],
                    tag_filters_from: vec![],
                },
            },
        );

        let config = create_test_config(path_rules, HashMap::new());
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![]),           // Whitelisted by name
            create_test_tool("tool2", Some("comp2"), vec![]),  // Whitelisted by component_id
            create_test_tool("tool3", Some("comp3"), vec![]),  // Not whitelisted
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(result.len(), 2, "Should only include whitelisted tools");
        assert!(result.iter().any(|t| t.name == "tool1"));
        assert!(result.iter().any(|t| t.name == "tool2"));
        assert!(!result.iter().any(|t| t.name == "tool3"));
    }

    #[test]
    fn test_pipeline_blacklist_overrides_whitelist() {
        // CRITICAL: "Deny Trumps Allow" - blacklist must win
        let mut path_rules = HashMap::new();
        path_rules.insert(
            "/mcp".to_string(),
            AggregatedPathRule {
                whitelist: vec!["tool1".to_string(), "comp2".to_string()],
                blacklist: vec!["tool1".to_string()], // Blacklist tool1
                tag_filters: HashMap::new(),
                sources: RuleSources {
                    whitelist_from: vec![],
                    blacklist_from: vec![],
                    tag_filters_from: vec![],
                },
            },
        );

        let config = create_test_config(path_rules, HashMap::new());
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![]),          // Whitelisted BUT blacklisted
            create_test_tool("tool2", Some("comp2"), vec![]), // Only whitelisted
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(result.len(), 1, "Blacklist should override whitelist");
        assert!(!result.iter().any(|t| t.name == "tool1"));
        assert!(result.iter().any(|t| t.name == "tool2"));
    }

    #[test]
    fn test_pipeline_tag_filters_global_only() {
        let mut global_filters = HashMap::new();
        global_filters.insert("category".to_string(), vec!["math".to_string()]);

        let config = create_test_config(HashMap::new(), global_filters);
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![("category", "math")]),    // Matches
            create_test_tool("tool2", None, vec![("category", "science")]), // Doesn't match
            create_test_tool("tool3", None, vec![]),                        // No tags
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(result.len(), 1, "Should only include tools matching tag filters");
        assert!(result.iter().any(|t| t.name == "tool1"));
    }

    #[test]
    fn test_pipeline_tag_filters_path_overrides_global_for_same_key() {
        // Path-specific tag filters should override global filters for the SAME key
        let mut global_filters = HashMap::new();
        global_filters.insert("category".to_string(), vec!["math".to_string()]);

        let mut path_rules = HashMap::new();
        path_rules.insert(
            "/mcp".to_string(),
            AggregatedPathRule {
                whitelist: vec![],
                blacklist: vec![],
                tag_filters: {
                    let mut map = HashMap::new();
                    // Override "category" from global
                    map.insert("category".to_string(), vec!["science".to_string()]);
                    map
                },
                sources: RuleSources {
                    whitelist_from: vec![],
                    blacklist_from: vec![],
                    tag_filters_from: vec![],
                },
            },
        );

        let config = create_test_config(path_rules, global_filters);
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![("category", "math")]),    // Would match global, but path overrides
            create_test_tool("tool2", None, vec![("category", "science")]), // Matches path override
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(
            result.len(),
            1,
            "Path tag filter should override global filter for same key"
        );
        assert!(result.iter().any(|t| t.name == "tool2"));
    }

    #[test]
    fn test_pipeline_tag_filters_merge_different_keys() {
        // Path and global filters with DIFFERENT keys should merge (AND logic)
        let mut global_filters = HashMap::new();
        global_filters.insert("category".to_string(), vec!["math".to_string()]);

        let mut path_rules = HashMap::new();
        path_rules.insert(
            "/mcp".to_string(),
            AggregatedPathRule {
                whitelist: vec![],
                blacklist: vec![],
                tag_filters: {
                    let mut map = HashMap::new();
                    // Add DIFFERENT key - should merge with global
                    map.insert("level".to_string(), vec!["basic".to_string()]);
                    map
                },
                sources: RuleSources {
                    whitelist_from: vec![],
                    blacklist_from: vec![],
                    tag_filters_from: vec![],
                },
            },
        );

        let config = create_test_config(path_rules, global_filters);
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![("category", "math")]),                      // Only has category
            create_test_tool("tool2", None, vec![("level", "basic")]),                        // Only has level
            create_test_tool("tool3", None, vec![("category", "math"), ("level", "basic")]),  // Has both - should match
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(
            result.len(),
            1,
            "Should require BOTH category=math AND level=basic"
        );
        assert!(result.iter().any(|t| t.name == "tool3"));
    }

    #[test]
    fn test_pipeline_combined_whitelist_blacklist_tags() {
        // Integration test: all three filter types together
        let mut global_filters = HashMap::new();
        global_filters.insert("level".to_string(), vec!["basic".to_string()]);

        let mut path_rules = HashMap::new();
        path_rules.insert(
            "/mcp".to_string(),
            AggregatedPathRule {
                whitelist: vec!["tool1".to_string(), "tool2".to_string(), "tool3".to_string()],
                blacklist: vec!["tool2".to_string()],
                tag_filters: HashMap::new(),
                sources: RuleSources {
                    whitelist_from: vec![],
                    blacklist_from: vec![],
                    tag_filters_from: vec![],
                },
            },
        );

        let config = create_test_config(path_rules, global_filters);
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string());

        let tools = vec![
            create_test_tool("tool1", None, vec![("level", "basic")]),    // Pass all filters
            create_test_tool("tool2", None, vec![("level", "basic")]),    // Blacklisted
            create_test_tool("tool3", None, vec![("level", "advanced")]), // Fails tag filter
            create_test_tool("tool4", None, vec![("level", "basic")]),    // Not whitelisted
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(result.len(), 1, "Should pass whitelist, blacklist, AND tag filters");
        assert!(result.iter().any(|t| t.name == "tool1"));
    }

    #[test]
    fn test_pipeline_no_path_match_uses_global_tags_only() {
        // When path doesn't match any rule, only global tag filters apply
        let mut global_filters = HashMap::new();
        global_filters.insert("category".to_string(), vec!["math".to_string()]);

        let mut path_rules = HashMap::new();
        path_rules.insert(
            "/other".to_string(),
            AggregatedPathRule {
                whitelist: vec!["blocked_tool".to_string()],
                blacklist: vec![],
                tag_filters: HashMap::new(),
                sources: RuleSources {
                    whitelist_from: vec![],
                    blacklist_from: vec![],
                    tag_filters_from: vec![],
                },
            },
        );

        let config = create_test_config(path_rules, global_filters);
        let pipeline = FilteringPipeline::new(&config, "/mcp".to_string()); // Different path

        let tools = vec![
            create_test_tool("tool1", None, vec![("category", "math")]),    // Matches global
            create_test_tool("tool2", None, vec![("category", "science")]), // Doesn't match
            create_test_tool("blocked_tool", None, vec![("category", "math")]), // Would be blocked on /other, but allowed here
        ];

        let result = pipeline.apply_filters(&tools);
        assert_eq!(result.len(), 2, "Should apply only global filters when path doesn't match");
        assert!(result.iter().any(|t| t.name == "tool1"));
        assert!(result.iter().any(|t| t.name == "blocked_tool"));
    }
}
