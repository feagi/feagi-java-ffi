## FEAGI Java FFI (Rust `cdylib`)

This folder hosts the **Rust-backed native library** that the future FEAGI Java SDK will call via **JNI**.

### Design goals
- **Stable ABI**: C ABI with opaque handles (`*Handle`) to avoid leaking Rust types.
- **Deterministic config**: endpoints/timeouts/retries must be provided explicitly by the caller.
- **Bytes & JSON only** across the boundary: no Rust structs cross FFI.
- **Android-ready later**: this is a `cdylib` with a plain C ABI, so it can be built for Android NDK targets in the future without changing the API.

### Public header
- `include/feagi_java_ffi.h`

### ABI compatibility
- Call `feagi_abi_version()` right after loading the native library.
- The Java bindings should refuse to run if the ABI version does not match.

### What exists today (minimal usable surface)
- **Config + client lifecycle**: create config, set explicit endpoints/timeouts/capabilities, create client, connect, send/receive.
- **Error reporting**: per-thread last error message via `feagi_last_error_message_alloc()` + `feagi_string_free()`.
- **Registration response helpers** (JSON): functions to retrieve the last successful registration body, derived ZMQ ports from transport config, and chosen transport.
- **Motor payload return**: motor data is returned as an opaque byte buffer handle (`FeagiByteBufferHandle`) to keep ownership correct across FFI.

### Build (local)
From repo root:

```bash
cd feagi-java-ffi
cargo build --release
```

The produced shared library will be under `target/release/` (platform-specific extension).

### Next steps (to resume later)
- **Publishing**: build and publish platform-specific native libs as Maven classifier artifacts (or attach to GitHub Releases).
- **JNI bridge**: implement a small JNI shim in the Java SDK that calls this C ABI (this library is C ABI, not JNI-exported symbols).
- **ABI stability guard**: add a CI check that validates exported symbols + ABI version changes are deliberate.
- **Capability ergonomics**: expand the C ABI to cover the minimal “common capabilities” you want to support first (vision + motor are already present).
