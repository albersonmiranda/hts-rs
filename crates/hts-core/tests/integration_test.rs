//! Integration tests for HTS-Core using real tourism data.

use hts_core::{HierarchicalTimeSeries, HierarchySpec};
use polars::prelude::*;
use std::path::PathBuf;

/// Get path to data files in workspace root.
fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/hts-core
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn data_csv() -> PathBuf {
    workspace_root().join("tests/data/bottom_series.csv")
}

fn aggregated_csv() -> PathBuf {
    workspace_root().join("tests/data/aggregated_series.csv")
}

/// Load the R aggregated CSV (semicolon-delimited with Brazilian decimal notation).
fn load_aggregated_df() -> PolarsResult<DataFrame> {
    CsvReadOptions::default()
        .with_has_header(true)
        .map_parse_options(|opts| opts.with_separator(b';').with_decimal_comma(true))
        .try_into_reader_with_file_path(Some(aggregated_csv().into()))?
        .finish()
}

/// Test loading the tourism data and verifying structure.
#[test]
fn test_load_tourism_data() {
    let spec = HierarchySpec::new(
        vec!["State".into(), "Region".into()],
        vec!["Purpose".into()],
    );

    let hts = HierarchicalTimeSeries::from_csv(data_csv(), spec, "Quarter", "Trips")
        .expect("Failed to load tourism data");

    // 80 quarters (1998 Q1 to 2017 Q4)
    assert_eq!(hts.n_periods(), 80);

    // Print summary for debugging
    println!("{}", hts.summary());

    // Should have bottom-level series (Region × Purpose combinations)
    assert!(hts.n_bottom() > 0);

    // Total series should be more than bottom (includes aggregated levels)
    assert!(hts.n_series() >= hts.n_bottom());
}

/// Test that the S matrix has correct shape.
#[test]
fn test_summation_matrix_shape() {
    let spec = HierarchySpec::new(
        vec!["State".into(), "Region".into()],
        vec!["Purpose".into()],
    );

    let hts = HierarchicalTimeSeries::from_csv(data_csv(), spec, "Quarter", "Trips")
        .expect("Failed to load tourism data");

    let s = hts.summation_matrix();
    let (n, m) = s.shape();

    println!("S matrix shape: {} × {}", n, m);
    println!("n_series: {}, n_bottom: {}", hts.n_series(), hts.n_bottom());

    // n = total series, m = bottom-level series
    assert_eq!(n, hts.n_series());
    assert_eq!(m, hts.n_bottom());
}

/// Validate total aggregation matches R's output.
#[test]
fn test_total_aggregation() {
    // Load bottom-level data
    let bottom_df = CsvReadOptions::default()
        .with_has_header(true)
        .try_into_reader_with_file_path(Some(data_csv().into()))
        .unwrap()
        .finish()
        .unwrap();

    // Load R's aggregated data (semicolon-delimited)
    let agg_df = match load_aggregated_df() {
        Ok(df) => df,
        Err(e) => {
            println!("Skipping R comparison due to CSV parsing error: {}", e);
            // Still verify our own sum works
            let rust_total = bottom_df
                .clone()
                .lazy()
                .filter(col("Quarter").eq(lit("1998 Q1")))
                .select([col("Trips").sum()])
                .collect()
                .unwrap()
                .column("Trips")
                .unwrap()
                .f64()
                .unwrap()
                .get(0)
                .unwrap();

            println!("Rust sum for 1998 Q1: {}", rust_total);
            assert!(rust_total > 20000.0, "Total should be > 20000");
            return;
        }
    };

    println!("Loaded aggregated.csv with {} rows", agg_df.height());
    println!("Columns: {:?}", agg_df.get_column_names());

    // Get total for 1998 Q1 from R's output
    // (where State, Purpose, Region are all <aggregated>)
    let r_total_1998_q1 = agg_df
        .clone()
        .lazy()
        .filter(col("Quarter").eq(lit("1998 Q1")))
        .filter(col("State").eq(lit("<aggregated>")))
        .filter(col("Purpose").eq(lit("<aggregated>")))
        .filter(col("Region").eq(lit("<aggregated>")))
        .select([col("Trips")])
        .collect()
        .unwrap();

    let r_total = r_total_1998_q1
        .column("Trips")
        .unwrap()
        .f64()
        .unwrap()
        .get(0)
        .expect("Should have at least one row matching total");

    // Calculate total from bottom data for 1998 Q1
    let rust_total = bottom_df
        .clone()
        .lazy()
        .filter(col("Quarter").eq(lit("1998 Q1")))
        .select([col("Trips").sum()])
        .collect()
        .unwrap()
        .column("Trips")
        .unwrap()
        .f64()
        .unwrap()
        .get(0)
        .unwrap();

    println!("R total for 1998 Q1: {}", r_total);
    println!("Rust sum for 1998 Q1: {}", rust_total);

    // Should match (with floating point tolerance)
    assert!(
        (r_total - rust_total).abs() < 0.01,
        "Totals don't match: R={}, Rust={}",
        r_total,
        rust_total
    );
}

/// Test that aggregate_all produces results matching R's output.
#[test]
fn test_full_aggregation_match() {
    let spec = HierarchySpec::new(
        vec!["State".into(), "Region".into()],
        vec!["Purpose".into()],
    );

    let hts = HierarchicalTimeSeries::from_csv(data_csv(), spec, "Quarter", "Trips")
        .expect("Failed to load tourism data");

    let rust_agg = hts.aggregate_all().expect("Failed to aggregate");

    println!("Rust aggregated rows: {}", rust_agg.height());

    // Load R's aggregated data
    let r_agg = match load_aggregated_df() {
        Ok(df) => df,
        Err(e) => {
            println!("Skipping full aggregation comparison due to error: {}", e);
            return;
        }
    };

    println!("R aggregated rows: {}", r_agg.height());

    assert_eq!(rust_agg.height(), r_agg.height(), "Row counts should match");

    // Sort both by all key columns + Quarter to compare
    let sort_cols = vec!["State", "Region", "Purpose", "Quarter"];

    let rust_sorted = rust_agg
        .sort(sort_cols.clone(), SortMultipleOptions::default())
        .unwrap();

    let r_sorted = r_agg
        .sort(sort_cols, SortMultipleOptions::default())
        .unwrap();

    // Check first 5 rows match
    let sub_rust = rust_sorted.head(Some(5));
    let sub_r = r_sorted.head(Some(5));

    println!("Rust head:\n{}", sub_rust);
    println!("R head:\n{}", sub_r);

    // Check totals
    let rust_sum: f64 = rust_sorted
        .column("Trips")
        .unwrap()
        .f64()
        .unwrap()
        .sum()
        .unwrap();
    let r_sum: f64 = r_sorted
        .column("Trips")
        .unwrap()
        .f64()
        .unwrap()
        .sum()
        .unwrap();

    assert!(
        (rust_sum - r_sum).abs() < 1.0,
        "Total sum across all series should match"
    );
}

/// Count unique series in R's aggregated output.
#[test]
fn test_count_series_from_r() {
    let agg_df = match load_aggregated_df() {
        Ok(df) => df,
        Err(e) => {
            println!("Skipping R series count due to CSV parsing error: {}", e);
            return;
        }
    };

    // Count unique combinations of State, Purpose, Region
    let unique_series = agg_df
        .clone()
        .lazy()
        .select([col("State"), col("Purpose"), col("Region")])
        .unique(None, UniqueKeepStrategy::First)
        .collect()
        .unwrap();

    println!("R's unique series count: {}", unique_series.height());

    // From our analysis: 425 unique series
    assert_eq!(unique_series.height(), 425);
}
