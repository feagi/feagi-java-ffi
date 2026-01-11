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

### Build (local)
From repo root:

```bash
cd feagi-java-ffi
cargo build --release
```

The produced shared library will be under `target/release/` (platform-specific extension).

