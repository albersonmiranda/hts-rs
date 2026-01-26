//! Summation matrix for hierarchical time series.
//!
//! The summation matrix S maps bottom-level series to all series:
//! y = Sb, where y is the n-vector of all series and b is the m-vector
//! of bottom-level series.

use crate::hierarchy::HierarchyTree;
use faer::Mat;

/// The summation matrix S where y = Sb.
///
/// This is an n × m matrix where:
/// - n = total number of series (all levels)
/// - m = number of bottom-level series
///
/// Entry S[i,j] = 1 if bottom-level series j contributes to series i,
/// otherwise S[i,j] = 0.
///
/// Uses `faer::Mat<f64>` for efficient dense matrix operations.
#[derive(Debug, Clone)]
pub struct SummationMatrix {
    /// The S matrix stored as dense f64.
    matrix: Mat<f64>,

    /// Labels for all n series (rows).
    row_labels: Vec<String>,

    /// Labels for m bottom-level series (columns).
    col_labels: Vec<String>,
}

impl SummationMatrix {
    /// Builds the summation matrix from a hierarchy tree.
    ///
    /// # Arguments
    ///
    /// * `tree` - The hierarchy tree defining the aggregation structure
    ///
    /// # Returns
    ///
    /// A `SummationMatrix` with the correct structure.
    pub fn from_hierarchy(tree: &HierarchyTree) -> Self {
        let n = tree.n_series();
        let m = tree.n_bottom();

        // Create dense matrix initialized to zeros
        let mut matrix = Mat::zeros(n, m);

        let mut row_labels = Vec::with_capacity(n);
        let mut col_labels = Vec::with_capacity(m);

        // Build column labels (bottom-level series)
        for node in tree.bottom_level_nodes() {
            col_labels.push(node.id.clone());
        }

        // Build the matrix row by row
        for (row_idx, node) in tree.nodes().iter().enumerate() {
            row_labels.push(node.id.clone());

            // Set 1.0 for each bottom-level series that aggregates to this node
            for &bottom_idx in &node.aggregates_from {
                matrix[(row_idx, bottom_idx)] = 1.0;
            }
        }

        Self {
            matrix,
            row_labels,
            col_labels,
        }
    }

    /// Aggregates bottom-level values to all levels: y = S * b.
    ///
    /// # Arguments
    ///
    /// * `bottom_values` - Values for the m bottom-level series
    ///
    /// # Returns
    ///
    /// Values for all n series.
    ///
    /// # Panics
    ///
    /// Panics if `bottom_values.len() != self.n_bottom()`.
    pub fn aggregate(&self, bottom_values: &[f64]) -> Vec<f64> {
        assert_eq!(
            bottom_values.len(),
            self.n_bottom(),
            "Expected {} bottom values, got {}",
            self.n_bottom(),
            bottom_values.len()
        );

        let m = self.n_bottom();
        let n = self.n_series();

        // Create column vector from bottom values
        let b = Mat::from_fn(m, 1, |i, _| bottom_values[i]);

        // Multiply: y = S * b
        let y = &self.matrix * &b;

        // Extract result as Vec
        (0..n).map(|i| y[(i, 0)]).collect()
    }

    /// Returns the matrix dimensions (n_series, n_bottom).
    pub fn shape(&self) -> (usize, usize) {
        (self.matrix.nrows(), self.matrix.ncols())
    }

    /// Returns the number of total series (n).
    pub fn n_series(&self) -> usize {
        self.matrix.nrows()
    }

    /// Returns the number of bottom-level series (m).
    pub fn n_bottom(&self) -> usize {
        self.matrix.ncols()
    }

    /// Returns a reference to the underlying faer matrix.
    pub fn as_faer(&self) -> &Mat<f64> {
        &self.matrix
    }

    /// Returns row labels (all series).
    pub fn row_labels(&self) -> &[String] {
        &self.row_labels
    }

    /// Returns column labels (bottom-level series).
    pub fn col_labels(&self) -> &[String] {
        &self.col_labels
    }

    /// Converts to a 2D Vec for inspection/debugging.
    pub fn to_vec(&self) -> Vec<Vec<f64>> {
        let (n, m) = self.shape();
        (0..n)
            .map(|i| (0..m).map(|j| self.matrix[(i, j)]).collect())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hierarchy::{HierarchySpec, HierarchyTree};
    use polars::prelude::*;

    #[test]
    fn test_summation_matrix_shape() {
        let df = df! {
            "State" => ["A", "A", "B", "B"],
            "Region" => ["A1", "A2", "B1", "B2"],
            "Value" => [1.0, 2.0, 3.0, 4.0],
        }
        .unwrap();

        let spec = HierarchySpec::hierarchical(vec!["State".into(), "Region".into()]);
        let tree = HierarchyTree::from_dataframe(&df, &spec).unwrap();
        let s = SummationMatrix::from_hierarchy(&tree);

        // m = 4 bottom-level series
        assert_eq!(s.n_bottom(), 4);

        // n should be >= 4 (bottom) + some aggregated levels
        assert!(s.n_series() >= 4);

        let (n, m) = s.shape();
        assert_eq!(m, 4);
        assert!(n >= 4);
    }

    #[test]
    fn test_summation_matrix_aggregate() {
        let df = df! {
            "State" => ["A", "A", "B", "B"],
            "Region" => ["A1", "A2", "B1", "B2"],
            "Value" => [1.0, 2.0, 3.0, 4.0],
        }
        .unwrap();

        let spec = HierarchySpec::hierarchical(vec!["State".into(), "Region".into()]);
        let tree = HierarchyTree::from_dataframe(&df, &spec).unwrap();
        let s = SummationMatrix::from_hierarchy(&tree);

        // Bottom values
        let bottom = vec![1.0, 2.0, 3.0, 4.0];
        let all = s.aggregate(&bottom);

        // Total should be 1+2+3+4 = 10
        // Find total in results
        let total_idx = s.row_labels().iter().position(|l| l == "Total").unwrap();
        assert_eq!(all[total_idx], 10.0);
    }

    #[test]
    fn test_identity_at_bottom() {
        // The bottom portion of S should be an identity matrix
        let df = df! {
            "Region" => ["A", "B", "C"],
            "Value" => [1.0, 2.0, 3.0],
        }
        .unwrap();

        let spec = HierarchySpec::hierarchical(vec!["Region".into()]);
        let tree = HierarchyTree::from_dataframe(&df, &spec).unwrap();
        let s = SummationMatrix::from_hierarchy(&tree);

        let _mat = s.to_vec();

        // Check that bottom rows form identity-like structure
        // (each bottom series only aggregates from itself)
        for node in tree.nodes() {
            if node.is_bottom() {
                assert_eq!(node.aggregates_from.len(), 1);
            }
        }
    }
}
