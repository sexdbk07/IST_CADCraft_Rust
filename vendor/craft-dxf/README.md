# craft-dxf (vendored)

Copied from `crates/dxf` of storytold/craft-libs at
`219c1d0b14177af6f291395d6587f61a5e09043a` (MIT OR Apache-2.0), the revision this
workspace pins. Root `Cargo.toml` patches the craft-libs source to this path.

Changes from upstream:

- `normalize_hex` copies hex text that is all hex digits as it is instead of
  filtering it character by character. Binary chunks (groups 310-319, 1004) are
  most of a large DXF's bytes; the per-character path took 40% of a DWG save.

To be offered upstream to craft-libs; drop this copy once craft-libs has it.
