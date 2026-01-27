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

//! Example for hts-core crate showcasing basic usage for the README.

fn main() {
    use hts_core::{HierarchicalTimeSeries, HierarchySpec};
    use polars::prelude::*;
    use std::env;

    unsafe {
        env::set_var("POLARS_FMT_MAX_ROWS", "20");
    }

    // Data for the example (Brazilian GDP)
    // One can also load from a CSV file with `HierarchicalTimeSeries::from_csv()`
    let data = df!(
      "State" => &[
        vec!["Rio de Janeiro"; 4], vec!["São Paulo"; 4],
        vec!["Rio de Janeiro"; 4], vec!["São Paulo"; 4],
      ].concat(),
      "City" => &[
        vec!["Rio de Janeiro"; 2], vec!["Duque de Caxias"; 2],
        vec!["São Paulo"; 2], vec!["Campinas"; 2],
        vec!["Rio de Janeiro"; 2], vec!["Duque de Caxias"; 2],
        vec!["São Paulo"; 2], vec!["Campinas"; 2],
      ].concat(),
      "Sector" => vec!["Industry", "Agriculture"].repeat(8),
      "Quarter" => [vec!["2024 Q1"; 8], vec!["2024 Q2"; 8]].concat(),
      "GDP" => &[
        1000, 500, 150, 120,
        2000, 800, 300, 200,
        1500, 800, 200, 150,
        2200, 900, 400, 300,
      ],
    )
    .unwrap();

    println!("Bottom-level data:\n{}", data);

    // Define the structure
    let spec = HierarchySpec::new(vec!["State".into(), "City".into()], vec!["Sector".into()]);

    // Load data
    let hts = HierarchicalTimeSeries::new(data, spec, "Quarter", "GDP").unwrap();

    // Access the summation matrix
    let s = hts.summation_matrix();
    println!("S matrix shape: {:?}", s.shape());

    // Print the summation matrix
    println!("S matrix:\n{:?}", s);

    // Print summary
    println!("{}", hts.summary());

    // Aggregate
    let aggregated = hts.aggregate_all().unwrap();
    println!("Aggregated Data:\n{}", aggregated);
}
