# hts-rs

![Status](https://img.shields.io/badge/status-experimental-orange)
[![License](https://img.shields.io/badge/license-GPLv3-blue)](LICENSE)

**Hierarchical and Grouped Time Series in Rust**

`hts-rs` is a high-performance library for working with hierarchical and grouped time series data. It provides efficient data structures and algorithms for aggregating time series and constructing summation matrices for forecast reconciliation.

## Features

- **Flexible Hierarchy Specification**: Define complex nested hierarchies (e.g., State → Region) and crossed grouping variables (e.g., Purpose).
- **Efficient Aggregation**: Fast bottom-up aggregation using `Polars`.
- **Dense Matrix Support**: optimized storage for the summation matrix ($\mathbf{S}$) used in reconciliation (via `faer`).
- **Time Frequency Agnostic**: Built-in support for Annual, Quarterly, Monthly, Weekly, and Daily data.
- **Polars Integration**: Seamlessly works with Polars DataFrames for data input and output.

## Installation

Add `hts-core` to your `Cargo.toml`:

```toml
[dependencies]
hts-core = { git = "https://github.com/albersonmiranda/hts-rs" }
```

*Note: This crate is currently experimental and not yet published to crates.io.*

## Usage

### Defining a Hierarchy

```rust
use hts_core::{HierarchicalTimeSeries, HierarchySpec};

// Define the structure:
// - Nested: Region belongs to State
// - Crossed: Purpose (Business, Holiday, etc.) occurs at every level
let spec = HierarchySpec::new(
    vec!["State".into(), "Region".into()],
    vec!["Purpose".into()],
);
```

### Loading and Aggregating Data

```rust
// Load data from a CSV file
// The crate automatically detects time frequency (e.g., "1998 Q1", "2023-01-01")
let hts = HierarchicalTimeSeries::from_csv(
    "tourism_data.csv",
    spec,
    "Quarter", // Time column
    "Trips",   // Value column
)?;

// Generate a DataFrame with ALL aggregation levels (Total, State, Purpose, State*Purpose, etc.)
let full_df = hts.aggregate_all()?;

println!("{}", full_df);
```

### Accessing the Summation Matrix

For forecast reconciliation (computation of $\tilde{\mathbf{y}} = \mathbf{S}\mathbf{P}\hat{\mathbf{y}}$):

```rust
let s_matrix = hts.summation_matrix();
println!("S matrix shape: {:?}", s_matrix.shape());
```

## Supported Time Frequencies

The parser automatically detects standard string formats:

- **Annual**: `"2024"`
- **Quarterly**: `"2024 Q1"`
- **Monthly**: `"2024 M01"`
- **Weekly**: `"2024 W01"`
- **Daily**: `"2024-01-01"`

## Roadmap

- [x] **Core Structures**: Hierarchy tree, Node, Summation Matrix
- [x] **Aggregation Engine**: `aggregate_all` implementation
- [ ] **Reconciliation Algorithms**
- [ ] **R bindings**

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License.
