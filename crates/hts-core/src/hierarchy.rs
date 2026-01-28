// Copyright (C) 2026 Alberson Miranda
//
// This file is part of hts-rs.
//
// hts-rs is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// hts-rs is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with hts-rs.  If not, see <https://www.gnu.org/licenses/>.

//! Hierarchy specification and tree structures.
//!
//! This module defines how hierarchical and grouped time series structures
//! are specified and represented internally.

use crate::error::{HtsError, Result};
use polars::prelude::*;
use std::collections::{HashMap, HashSet};

/// Specification of hierarchical and grouped structure.
///
/// # Hierarchical vs Grouped
///
/// - **Hierarchical columns**: Strict parent-child nesting where each child
///   belongs to exactly one parent. Example: `["State", "Region"]` means
///   each Region belongs to exactly one State.
///
/// - **Grouped columns**: Crossed dimensions that combine with all levels
///   of the hierarchy. Example: `["Purpose"]` means each Purpose appears
///   at every hierarchical level.
///
/// # Example
///
/// ```
/// use hts_core::HierarchySpec;
///
/// let spec = HierarchySpec {
///     hierarchy: vec!["State".into(), "Region".into()],
///     groups: vec!["Purpose".into()],
/// };
/// ```

#[derive(Debug, Clone, Default)]
pub struct HierarchySpec {
    /// Columns with strict parent-child nesting, ordered from top to bottom.
    /// Each value at level i belongs to exactly one value at level i-1.
    pub hierarchy: Vec<String>,

    /// Columns that cross with the hierarchy at all levels.
    /// These create additional aggregation dimensions.
    pub groups: Vec<String>,
}

impl HierarchySpec {
    /// Creates a new `HierarchySpec`.
    pub fn new(hierarchy: Vec<String>, groups: Vec<String>) -> Self {
        Self { hierarchy, groups }
    }

    /// Creates a spec with only hierarchical columns (no grouping).
    pub fn hierarchical(columns: Vec<String>) -> Self {
        Self {
            hierarchy: columns,
            groups: Vec::new(),
        }
    }

    /// Creates a spec with only grouped columns (no hierarchy).
    pub fn grouped(columns: Vec<String>) -> Self {
        Self {
            hierarchy: Vec::new(),
            groups: columns,
        }
    }

    /// Returns all columns involved in the structure.
    pub fn all_columns(&self) -> Vec<&str> {
        self.hierarchy
            .iter()
            .chain(self.groups.iter())
            .map(String::as_str)
            .collect()
    }

    /// Returns all combinations of columns that define aggregation levels.
    ///
    /// Includes the root (empty), hierarchical levels, and crossed levels with groups.
    pub fn level_combinations(&self) -> Vec<Vec<String>> {
        let n_hier = self.hierarchy.len();
        let has_groups = !self.groups.is_empty();
        let mut all_level_keys: Vec<Vec<String>> = Vec::new();

        // Level 0: Total
        all_level_keys.push(Vec::new());

        // For each hierarchical level
        for h in 0..=n_hier {
            let hier_cols: Vec<String> = self.hierarchy[..h].to_vec();

            if h > 0 {
                // Hierarchy only at this level
                all_level_keys.push(hier_cols.clone());
            }

            // If we have groups, add crossed versions
            if has_groups && h < n_hier {
                // Groups only (at level 1 if no hierarchy yet)
                if h == 0 {
                    all_level_keys.push(self.groups.clone());
                }
                // Hierarchy × Groups
                let mut crossed = hier_cols.clone();
                crossed.extend(self.groups.clone());
                if !crossed.is_empty() && h > 0 {
                    all_level_keys.push(crossed);
                }
            }
        }

        // Bottom level: all columns
        let all_cols: Vec<String> = self.all_columns().iter().map(|s| s.to_string()).collect();
        if !all_level_keys.contains(&all_cols) {
            all_level_keys.push(all_cols);
        }

        // Deduplicate and sort by number of columns (level)
        let mut seen = HashSet::new();
        all_level_keys.retain(|k| {
            let key = k.join("/");
            if seen.contains(&key) {
                false
            } else {
                seen.insert(key);
                true
            }
        });
        all_level_keys.sort_by_key(|k| k.len());

        all_level_keys
    }

    /// Validates that all specified columns exist in the DataFrame.
    pub fn validate(&self, df: &DataFrame) -> Result<()> {
        let df_cols: HashSet<String> = df
            .get_column_names()
            .into_iter()
            .map(|s| s.to_string())
            .collect();

        for col in self.all_columns() {
            if !df_cols.contains(col) {
                return Err(HtsError::ColumnNotFound(col.to_string()));
            }
        }

        Ok(())
    }
}

/// A node representing one series in the hierarchy.
///
/// Each node corresponds to a single time series at some level of aggregation.
#[derive(Debug, Clone)]
pub struct Node {
    /// Unique identifier for this node (e.g., "South Australia/Adelaide/Business").
    pub id: String,

    /// Level in the hierarchy (0 = total, higher = more disaggregated).
    pub level: usize,

    /// Indices of bottom-level series that aggregate to this node.
    pub aggregates_from: Vec<usize>,

    /// Labels for the grouping keys at this node.
    pub labels: HashMap<String, String>,
}

impl Node {
    /// Creates a new node.
    pub fn new(id: String, level: usize) -> Self {
        Self {
            id,
            level,
            aggregates_from: Vec::new(),
            labels: HashMap::new(),
        }
    }

    /// Returns true if this is a bottom-level (most disaggregated) node.
    pub fn is_bottom(&self) -> bool {
        self.aggregates_from.len() == 1
    }
}

/// The hierarchy tree containing all aggregation levels.
///
/// This structure holds all nodes from the total (top) level down to
/// the bottom (most disaggregated) level, along with the aggregation
/// relationships needed to build the summation matrix.
#[derive(Debug, Clone)]
pub struct HierarchyTree {
    /// All nodes in the tree, ordered by level then by ID.
    nodes: Vec<Node>,

    /// Number of bottom-level series.
    n_bottom: usize,

    /// Number of levels in the hierarchy.
    n_levels: usize,

    /// Map from node ID to index in `nodes`.
    id_to_index: HashMap<String, usize>,
}

impl HierarchyTree {
    /// Builds a hierarchy tree from a DataFrame and specification.
    ///
    /// # Arguments
    ///
    /// * `df` - DataFrame containing the bottom-level data
    /// * `spec` - Hierarchy specification defining the structure
    ///
    /// # Returns
    ///
    /// A `HierarchyTree` with all aggregation levels computed.
    pub fn from_dataframe(df: &DataFrame, spec: &HierarchySpec) -> Result<Self> {
        spec.validate(df)?;

        // Get unique combinations of all grouping columns (bottom level)
        let all_cols = spec.all_columns();
        let bottom_df = df
            .clone()
            .lazy()
            .select(all_cols.iter().map(|c| col(*c)).collect::<Vec<_>>())
            .unique(None, UniqueKeepStrategy::First)
            .sort(all_cols.clone(), SortMultipleOptions::default())
            .collect()?;

        let n_bottom = bottom_df.height();

        // Build bottom-level nodes first
        let mut nodes = Vec::new();
        let mut id_to_index = HashMap::new();

        // Get all level definitions
        let all_level_keys = spec.level_combinations();

        // Build nodes for each level
        let n_levels = all_level_keys.len();

        for (level, level_cols) in all_level_keys.iter().enumerate() {
            if level_cols.is_empty() {
                // Total node
                let mut node = Node::new("Total".to_string(), level);
                node.aggregates_from = (0..n_bottom).collect();
                id_to_index.insert(node.id.clone(), nodes.len());
                nodes.push(node);
            } else {
                // Get unique combinations at this level
                let unique_df = bottom_df
                    .clone()
                    .lazy()
                    .select(
                        level_cols
                            .iter()
                            .map(|c| col(c.as_str()))
                            .collect::<Vec<_>>(),
                    )
                    .unique(None, UniqueKeepStrategy::First)
                    .sort(
                        level_cols
                            .iter()
                            .map(|name| name.as_str())
                            .collect::<Vec<_>>(),
                        SortMultipleOptions::default(),
                    )
                    .collect()?;

                for row_idx in 0..unique_df.height() {
                    let mut labels = HashMap::new();
                    let mut id_parts = Vec::new();

                    for col_name in level_cols {
                        let series = unique_df.column(col_name)?;
                        let value = series.get(row_idx)?.to_string();
                        let value = value.trim_matches('"').to_string();
                        labels.insert(col_name.clone(), value.clone());
                        id_parts.push(value);
                    }

                    let id = id_parts.join("/");
                    let mut node = Node::new(id.clone(), level);
                    node.labels = labels.clone();

                    // Find which bottom-level indices aggregate to this node
                    for bottom_idx in 0..n_bottom {
                        let matches = level_cols.iter().all(|col_name| {
                            let bottom_series = bottom_df.column(col_name).unwrap();
                            let bottom_val = bottom_series.get(bottom_idx).unwrap().to_string();
                            let bottom_val = bottom_val.trim_matches('"');
                            labels.get(col_name).map(|v| v.as_str()) == Some(bottom_val)
                        });

                        if matches {
                            node.aggregates_from.push(bottom_idx);
                        }
                    }

                    id_to_index.insert(id, nodes.len());
                    nodes.push(node);
                }
            }
        }

        Ok(Self {
            nodes,
            n_bottom,
            n_levels,
            id_to_index,
        })
    }

    /// Returns the total number of series (all levels).
    pub fn n_series(&self) -> usize {
        self.nodes.len()
    }

    /// Returns the number of bottom-level series.
    pub fn n_bottom(&self) -> usize {
        self.n_bottom
    }

    /// Returns the number of levels in the hierarchy.
    pub fn n_levels(&self) -> usize {
        self.n_levels
    }

    /// Returns all nodes.
    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    /// Returns the node with the given ID, if it exists.
    pub fn get_node(&self, id: &str) -> Option<&Node> {
        self.id_to_index.get(id).map(|&idx| &self.nodes[idx])
    }

    /// Returns an iterator over bottom-level nodes.
    pub fn bottom_level_nodes(&self) -> impl Iterator<Item = &Node> {
        self.nodes.iter().filter(|n| n.is_bottom())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_df() -> DataFrame {
        df! {
            "State" => ["A", "A", "B", "B"],
            "Region" => ["A1", "A2", "B1", "B2"],
            "Purpose" => ["X", "X", "X", "X"],
            "Value" => [1.0, 2.0, 3.0, 4.0],
        }
        .unwrap()
    }

    #[test]
    fn test_hierarchy_spec_validate() {
        let df = sample_df();
        let spec = HierarchySpec::new(
            vec!["State".into(), "Region".into()],
            vec!["Purpose".into()],
        );
        assert!(spec.validate(&df).is_ok());

        let bad_spec = HierarchySpec::new(vec!["NonExistent".into()], vec![]);
        assert!(bad_spec.validate(&df).is_err());
    }

    #[test]
    fn test_hierarchy_tree_simple() {
        let df = df! {
            "State" => ["A", "A", "B", "B"],
            "Region" => ["A1", "A2", "B1", "B2"],
            "Value" => [1.0, 2.0, 3.0, 4.0],
        }
        .unwrap();

        let spec = HierarchySpec::hierarchical(vec!["State".into(), "Region".into()]);
        let tree = HierarchyTree::from_dataframe(&df, &spec).unwrap();

        // Should have: Total (1) + States (2) + Regions (4) = 7 nodes
        assert_eq!(tree.n_bottom(), 4);
        assert!(tree.n_series() >= 4); // At minimum bottom level

        // Total should aggregate all bottom series
        let total = tree.get_node("Total").unwrap();
        assert_eq!(total.aggregates_from.len(), 4);
    }
}
