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

//! # HTS-Core: Hierarchical Time Series data structures and operations.
//!
//! This crate provides the core functionality for working with hierarchical and grouped time series in Rust, implementing concepts from forecast reconciliation literature. It provides efficient data structures and algorithms for aggregating time series and constructing summation matrices for forecast reconciliation.
//!
//! ## Key Concepts
//!
//! - **Hierarchical time series**: Series that can be aggregated in a parent-child structure (e.g., Region → State → Country)
//! - **Grouped time series**: Series that cross with hierarchy at all levels (e.g., Product Category applies to all geographic levels)
//! - **Summation matrix (S)**: Maps bottom-level series to all aggregation levels via y = Sb
//!
//! ## Features
//!
//! - **Flexible Hierarchy Specification**: Define complex nested hierarchies (e.g., State -> Region) and crossed grouping variables (e.g., Purpose).
//! - **Efficient Aggregation**: Fast bottom-up aggregation using `Polars`.
//! - **Dense Matrix Support**: Optimized storage for the summation matrix (S) used in reconciliation.
//! - **Time Frequency Agnostic**: Built-in support for Annual, Quarterly, Monthly, Weekly, and Daily data.
//! - **Polars Integration**: Seamlessly works with Polars DataFrames for data input and output.

//! ## Examples
//!
//! ```rust
//! use hts_core::{HierarchicalTimeSeries, HierarchySpec};
//! use polars::prelude::*;
//!
//! // Data for the example (Brazilian GDP)
//! // One can also load from a CSV file with `HierarchicalTimeSeries::from_csv()`
//! let data = df!(
//!   "State" => &[
//!     vec!["Rio de Janeiro"; 4], vec!["São Paulo"; 4],
//!     vec!["Rio de Janeiro"; 4], vec!["São Paulo"; 4],
//!   ].concat(),
//!   "City" => &[
//!     vec!["Rio de Janeiro"; 2], vec!["Duque de Caxias"; 2],
//!     vec!["São Paulo"; 2], vec!["Campinas"; 2],
//!     vec!["Rio de Janeiro"; 2], vec!["Duque de Caxias"; 2],
//!     vec!["São Paulo"; 2], vec!["Campinas"; 2],
//!   ].concat(),
//!   "Sector" => vec!["Industry", "Agriculture"].repeat(8),
//!   "Quarter" => [vec!["2024 Q1"; 8], vec!["2024 Q2"; 8]].concat(),
//!   "GDP" => &[
//!     1000, 500, 150, 120,
//!     2000, 800, 300, 200,
//!     1500, 800, 200, 150,
//!     2200, 900, 400, 300,
//!   ],
//! )
//! .unwrap();
//!    
//! // Define the structure
//! let spec = HierarchySpec::new(vec!["State".into(), "City".into()], vec!["Sector".into()]);
//!
//! // Load data
//! let hts = HierarchicalTimeSeries::new(data, spec, "Quarter", "GDP").unwrap();
//!
//! // Access the summation matrix
//! let s = hts.summation_matrix();
//! println!("S matrix shape: {:?}", s.shape());
//! 
//! // # S matrix shape: (21, 8)
//!
//! // Print summary
//! println!("{}", hts.summary());
//! 
//! // # Hierarchical Time Series Summary
//! // # ================================
//! // # Total series:  21
//! // # Bottom series: 8
//! // # Time periods:  2
//! // # Hierarchy:     ["State", "City"]
//! // # Groups:        ["Sector"]
//! // # S matrix:      21 × 8
//! ```
//! # Supported Time Frequencies
//!
//! The parser automatically detects standard string formats:
//!
//! - **Annual**: `"2024"`
//! - **Quarterly**: `"2024 Q1"`
//! - **Monthly**: `"2024 M01"`
//! - **Weekly**: `"2024 W01"`
//! - **Daily**: `"2024-01-01"`

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
