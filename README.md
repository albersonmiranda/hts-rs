# hts-rs: Hierarchical and Grouped Time Series in Rust

![Status](https://img.shields.io/badge/status-experimental-orange)
[![License](https://img.shields.io/badge/license-GPLv3-blue)](LICENSE)

`hts-rs` is a family of Rust crates that implements tools for working with hierarchical and grouped time series.

## Crates

- `hts-core`: Core functionality for hierarchical and grouped time series. It provides efficient data structures and algorithms for time series hierarchies specification and aggregation, and constructing summation matrices ($\mathbf{S}$). See [hts-core](https://github.com/albersonmiranda/hts-rs/blob/main/crates/hts-core/README.md) for details.

## Features

- **Grouped and hierarchical time series data wrangling**: The crate `hts-core` provides tools for working with hierarchical, grouped or mixed time series data, including specifications of hierarchies, agnostic time periods, building up aggregated time series from bottom-level series and constructing the summation matrix ($\mathbf{S}$).
- **Dense Matrix Support**: Blazingly fast matrix computations via [faer](https://codeberg.org/sarah-quinones/faer).

## Installation

*While in early alpha, the crate is not yet published to crates.io. You can include it to your `Cargo.toml` directly from the GitHub repository.*

```toml
[dependencies]
hts-rs = { git = "https://github.com/albersonmiranda/hts-rs" }
```

## Contributing

Contributions are welcome! Please feel free to submit a PR.

## License

This project is licensed under the GPLv3 License.
