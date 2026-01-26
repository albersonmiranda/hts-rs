//! Hierarchical time series data structure.
//!
//! This module provides the main `HierarchicalTimeSeries` type that combines
//! bottom-level data with the hierarchy structure and summation matrix.

use crate::error::{HtsError, Result};
use crate::hierarchy::{HierarchySpec, HierarchyTree};
use crate::period::Period;
use crate::summation_matrix::SummationMatrix;
use polars::prelude::*;
use std::path::Path;

/// A hierarchical and/or grouped time series dataset.
///
/// This is the main data structure for working with hierarchical time series.
/// It holds the bottom-level data along with the computed hierarchy tree and
/// summation matrix.
///
/// # Example
///
/// ```no_run
/// use hts_core::{HierarchicalTimeSeries, HierarchySpec};
///
/// let spec = HierarchySpec::new(
///     vec!["State".into(), "Region".into()],
///     vec!["Purpose".into()],
/// );
///
/// let hts = HierarchicalTimeSeries::from_csv(
///     "data.csv",
///     spec,
///     "Quarter",
///     "Trips",
/// ).unwrap();
///
/// println!("Total series: {}", hts.n_series());
/// println!("Bottom series: {}", hts.n_bottom());
/// ```
#[derive(Debug, Clone)]
pub struct HierarchicalTimeSeries {
    /// Bottom-level data as a Polars DataFrame.
    bottom_data: DataFrame,

    /// Hierarchy specification.
    spec: HierarchySpec,

    /// Hierarchy tree with all aggregation levels.
    tree: HierarchyTree,

    /// Summation matrix S.
    s_matrix: SummationMatrix,

    /// Parsed time periods.
    periods: Vec<Period>,

    /// Name of the time column.
    time_col: String,

    /// Name of the value column.
    value_col: String,
}

impl HierarchicalTimeSeries {
    /// Creates a new `HierarchicalTimeSeries` from a DataFrame.
    ///
    /// # Arguments
    ///
    /// * `bottom_data` - DataFrame containing the bottom-level time series
    /// * `spec` - Hierarchy specification
    /// * `time_col` - Name of the time/period column
    /// * `value_col` - Name of the value column
    ///
    /// # Errors
    ///
    /// Returns an error if columns are missing or data is invalid.
    pub fn new(
        bottom_data: DataFrame,
        spec: HierarchySpec,
        time_col: &str,
        value_col: &str,
    ) -> Result<Self> {
        // Validate columns exist
        if bottom_data.column(time_col).is_err() {
            return Err(HtsError::ColumnNotFound(time_col.to_string()));
        }
        if bottom_data.column(value_col).is_err() {
            return Err(HtsError::ColumnNotFound(value_col.to_string()));
        }

        // Parse time periods
        let time_series = bottom_data.column(time_col)?;
        let periods = Self::parse_periods(time_series)?;

        // Build hierarchy tree
        let tree = HierarchyTree::from_dataframe(&bottom_data, &spec)?;

        // Build summation matrix
        let s_matrix = SummationMatrix::from_hierarchy(&tree);

        Ok(Self {
            bottom_data,
            spec,
            tree,
            s_matrix,
            periods,
            time_col: time_col.to_string(),
            value_col: value_col.to_string(),
        })
    }

    /// Loads hierarchical time series data from a CSV file.
    ///
    /// # Arguments
    ///
    /// * `path` - Path to the CSV file
    /// * `spec` - Hierarchy specification
    /// * `time_col` - Name of the time/period column
    /// * `value_col` - Name of the value column
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn from_csv<P: AsRef<Path>>(
        path: P,
        spec: HierarchySpec,
        time_col: &str,
        value_col: &str,
    ) -> Result<Self> {
        let df = CsvReadOptions::default()
            .with_has_header(true)
            .try_into_reader_with_file_path(Some(path.as_ref().into()))?
            .finish()?;

        Self::new(df, spec, time_col, value_col)
    }

    /// Parses time periods from a Series.
    fn parse_periods(series: &Column) -> Result<Vec<Period>> {
        let mut periods = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for i in 0..series.len() {
            let val = series.get(i)?;
            let s = val.to_string();
            let s = s.trim_matches('"');

            if !seen.contains(s) {
                let period = Period::parse(s)?;
                periods.push(period);
                seen.insert(s.to_string());
            }
        }

        periods.sort();
        Ok(periods)
    }

    /// Returns the total number of series (all aggregation levels).
    pub fn n_series(&self) -> usize {
        self.tree.n_series()
    }

    /// Returns the number of bottom-level series.
    pub fn n_bottom(&self) -> usize {
        self.tree.n_bottom()
    }

    /// Returns the number of time periods.
    pub fn n_periods(&self) -> usize {
        self.periods.len()
    }

    /// Returns the time periods.
    pub fn periods(&self) -> &[Period] {
        &self.periods
    }

    /// Returns the summation matrix.
    pub fn summation_matrix(&self) -> &SummationMatrix {
        &self.s_matrix
    }

    /// Returns the hierarchy tree.
    pub fn hierarchy_tree(&self) -> &HierarchyTree {
        &self.tree
    }

    /// Returns the hierarchy specification.
    pub fn spec(&self) -> &HierarchySpec {
        &self.spec
    }

    /// Returns the bottom-level data.
    pub fn bottom_data(&self) -> &DataFrame {
        &self.bottom_data
    }

    /// Aggregates the data to create a DataFrame with all levels.
    ///
    /// Returns a DataFrame with columns for each grouping key, time, and value,
    /// containing data for all aggregation levels.
    /// Aggregates the data to create a DataFrame with all levels.
    ///
    /// Returns a DataFrame with columns for each grouping key, time, and value,
    /// containing data for all aggregation levels. Missing columns at each level
    /// are filled with "<aggregated>".
    pub fn aggregate_all(&self) -> Result<DataFrame> {
        let all_cols = self.spec.all_columns();
        // Get all combinations of columns that define the levels
        let levels = self.spec.level_combinations();

        // We will collect lazy frames for each level and concat them
        let mut frames = Vec::new();

        for level_cols in levels {
            // Group by the current level columns + time
            let mut group_cols: Vec<Expr> = level_cols.iter().map(|c| col(c.as_str())).collect();
            group_cols.push(col(&self.time_col));

            let mut lf = self
                .bottom_data
                .clone()
                .lazy()
                .group_by(group_cols)
                .agg([col(&self.value_col).sum()]);

            // Add missing columns as literals "<aggregated>"
            for &col_name in &all_cols {
                if !level_cols.contains(&col_name.to_string()) {
                    lf = lf.with_column(lit("<aggregated>").alias(col_name));
                }
            }

            // Select columns in consistent order
            let mut select_cols: Vec<Expr> = all_cols.iter().map(|c| col(*c)).collect();
            select_cols.push(col(&self.time_col));
            select_cols.push(col(&self.value_col));

            frames.push(lf.select(select_cols));
        }

        // Concat all levels
        let concatenated = concat(frames, UnionArgs::default())?;

        // Collect into DataFrame
        let df = concatenated.collect()?;
        Ok(df)
    }

    /// Gets the values for a specific series across all time periods.
    ///
    /// # Arguments
    ///
    /// * `series_id` - The series identifier (e.g., "South Australia/Adelaide/Business")
    ///
    /// # Returns
    ///
    /// A vector of values for each time period, or None if series not found.
    pub fn get_series(&self, series_id: &str) -> Option<Vec<f64>> {
        let _node = self.tree.get_node(series_id)?;

        // This is a simplified implementation
        // Full version would extract and aggregate data by time period
        Some(vec![0.0; self.n_periods()])
    }

    /// Returns a summary of the hierarchical structure.
    pub fn summary(&self) -> HtsSummary {
        HtsSummary {
            n_series: self.n_series(),
            n_bottom: self.n_bottom(),
            n_periods: self.n_periods(),
            hierarchy_cols: self.spec.hierarchy.clone(),
            group_cols: self.spec.groups.clone(),
            s_matrix_shape: self.s_matrix.shape(),
        }
    }
}

/// Summary of a hierarchical time series structure.
#[derive(Debug, Clone)]
pub struct HtsSummary {
    /// Total number of series.
    pub n_series: usize,
    /// Number of bottom-level series.
    pub n_bottom: usize,
    /// Number of time periods.
    pub n_periods: usize,
    /// Hierarchical columns.
    pub hierarchy_cols: Vec<String>,
    /// Grouped columns.
    pub group_cols: Vec<String>,
    /// Shape of S matrix (n, m).
    pub s_matrix_shape: (usize, usize),
}

impl std::fmt::Display for HtsSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Hierarchical Time Series Summary")?;
        writeln!(f, "================================")?;
        writeln!(f, "Total series:  {}", self.n_series)?;
        writeln!(f, "Bottom series: {}", self.n_bottom)?;
        writeln!(f, "Time periods:  {}", self.n_periods)?;
        writeln!(f, "Hierarchy:     {:?}", self.hierarchy_cols)?;
        writeln!(f, "Groups:        {:?}", self.group_cols)?;
        writeln!(
            f,
            "S matrix:      {} × {}",
            self.s_matrix_shape.0, self.s_matrix_shape.1
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hts_from_dataframe() {
        let df = df! {
            "Quarter" => ["1998 Q1", "1998 Q2", "1998 Q1", "1998 Q2"],
            "State" => ["A", "A", "B", "B"],
            "Region" => ["AA", "AA", "BA", "BA"],
            "Value" => [1.0, 2.0, 3.0, 4.0],
        }
        .unwrap();

        let spec = HierarchySpec::hierarchical(vec!["State".into(), "Region".into()]);
        let hts = HierarchicalTimeSeries::new(df, spec, "Quarter", "Value").unwrap();

        let summary = hts.summary();
        println!("{}", summary);

        assert_eq!(hts.n_periods(), 2);
        assert!(hts.n_series() >= 2);
    }

    #[test]
    fn test_hts_summary() {
        let df = df! {
            "Quarter" => vec!["2024 M01"; 8],
            "State" => [vec!["A"; 4], vec!["B"; 4]].concat(),
            "Region" => [vec!["AA"; 2], vec!["AB"; 2], vec!["BA"; 2], vec!["BB"; 2]].concat(),
            "Purpose" => ["X", "Y", "X", "Y", "X", "Y", "X", "Y"],
            "Value" => [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0],
        }
        .unwrap();

        let spec = HierarchySpec::new(
            vec!["State".into(), "Region".into()],
            vec!["Purpose".into()],
        );
        let hts = HierarchicalTimeSeries::new(df, spec, "Quarter", "Value").unwrap();

        let summary = hts.summary();
        println!("{}", summary);

        assert!(summary.n_series >= 1);
        assert!(summary.n_bottom >= 1);
    }
}
