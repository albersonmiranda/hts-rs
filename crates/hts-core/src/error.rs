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

//! Error types for HTS-Core operations.

use thiserror::Error;

/// Errors that can occur in HTS-Core operations.
#[derive(Debug, Error)]
pub enum HtsError {
    /// Error reading or writing files.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Error from Polars operations.
    #[error("Polars error: {0}")]
    Polars(#[from] polars::error::PolarsError),

    /// Error parsing time period string.
    #[error("Invalid period format: {0}")]
    InvalidPeriod(String),

    /// Error in hierarchy specification.
    #[error("Hierarchy error: {0}")]
    Hierarchy(String),

    /// Column not found in DataFrame.
    #[error("Column not found: {0}")]
    ColumnNotFound(String),
}

/// Result type alias for HTS-Core operations.
pub type Result<T> = std::result::Result<T, HtsError>;
