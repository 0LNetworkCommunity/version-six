// Copyright (c) The Libra Core Contributors
// SPDX-License-Identifier: Apache-2.0
//! Loaded definition of code data used in runtime.
//!
//! This module contains the loaded definition of code data used in runtime.

pub mod struct_def;
pub mod types;

#[cfg(all(test, feature = "fuzzing"))]
mod type_prop_tests;
