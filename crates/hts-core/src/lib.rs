//! HTS-Core: Hierarchical Time Series data structures and operations.
//!
//! This crate provides the core functionality for working with hierarchical
//! and grouped time series in Rust, implementing concepts from forecast
//! reconciliation literature.
//!
//! # Key Concepts
//!
//! - **Hierarchical time series**: Series that can be aggregated in a
//!   parent-child structure (e.g., Region → State → Country)
//! - **Grouped time series**: Series that cross with hierarchy at all levels
//!   (e.g., Product Category applies to all geographic levels)
//! - **Summation matrix (S)**: Maps bottom-level series to all aggregation
//!   levels via y = Sb
//!
//! # Example
//!
//! ```no_run
//! use hts_core::{HierarchicalTimeSeries, HierarchySpec};
//!
//! // Define the structure: Region nested in State, Purpose crosses all levels
//! let spec = HierarchySpec::new(
//!     vec!["State".into(), "Region".into()],
//!     vec!["Purpose".into()],
//! );
//!
//! // Load data
//! let hts = HierarchicalTimeSeries::from_csv(
//!     "data.csv",
//!     spec,
//!     "Quarter",
//!     "Trips",
//! ).unwrap();
//!
//! // Access the summation matrix
//! let s = hts.summation_matrix();
//! println!("S matrix shape: {:?}", s.shape());
//!
//! // Print summary
//! println!("{}", hts.summary());
//! ```

pub mod error;
pub mod hierarchy;
pub mod hts;
pub mod period;
pub mod summation_matrix;

pub use error::{HtsError, Result};
pub use hierarchy::{HierarchySpec, HierarchyTree, Node};
pub use hts::{HierarchicalTimeSeries, HtsSummary};
pub use period::Period;
pub use summation_matrix::SummationMatrix;
