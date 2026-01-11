// Copyright 2026 Neuraville Inc.
// SPDX-License-Identifier: Apache-2.0
//
// @cursor:ffi-safe
// A minimal, stable C ABI facade intended to be called from JNI.
//
// Design constraints:
// - Deterministic, explicit configuration (no hidden endpoint selection)
// - Opaque handles across the ABI boundary
// - Byte/JSON boundaries only (no Rust structs crossing FFI)
// - No fallbacks inside the FFI layer: caller must provide required config

mod ffi;

pub use ffi::*;

