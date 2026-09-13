# velocity-msi

Clean-room MSI (Windows Installer) package generator written in Rust.

Creates Windows Installer packages with a from-scratch OLE V3 compound file writer. **No dependency on `cfb`, `msi`, or `rust-msi` crates.**

## Features

- **From-scratch OLE V3 writer** — generates MS-CFB (Compound Binary File) format with 512-byte sectors
- **MSI database generation** — creates system tables (_Tables, _Columns, _Validation), string pool, and SummaryInformation
- **Cabinet embedding** — embed MS-ZIP cabinet files as OLE streams
- **Stream name encoding** — MSI base-64 Unicode encoding for table streams
- **Validation** — parse and verify OLE structure of generated MSI files
- **Windows-1252 encoding** — proper string encoding for MSI compatibility

## Architecture

```
velocity-msi/
├── ole.rs          — OLE V3 compound file writer (MS-CFB, 512-byte sectors)
├── string_pool.rs  — String interning with Windows-1252 encoding
├── table.rs        — Table schema, column types, and serialization
├── summary.rs      — SummaryInformation (OLE Property Set format)
├── cabinet.rs      — MSCF cabinet builder with MSZIP compression
├── validate.rs     — OLE structure reader for validation
├── error.rs        — Error types
└── lib.rs          — MsiBuilder orchestration API
```

## Usage

```rust
use velocity_msi::{MsiBuilder, Column, Value};

let mut builder = MsiBuilder::new();
builder.set_title("My Product");
builder.set_author("My Company");
builder.set_template("x64", 1033);

// Create a table
builder.create_table("Property", vec![
    Column::build("Property").string(72).primary_key().build(),
    Column::build("Value").string(255).nullable().build(),
]).unwrap();

// Insert rows (strings are auto-interned)
builder.insert_rows("Property", vec![
    vec![Value::from("ProductName"), Value::from("My Product")],
    vec![Value::from("ProductVersion"), Value::from("1.0.0")],
]).unwrap();

// Embed a cabinet file
builder.add_stream("data.cab", cabinet_bytes);

// Build the MSI
let msi_data = builder.build().unwrap();
std::fs::write("output.msi", &msi_data).unwrap();
```

## Validation

```rust
use velocity_msi::validate_ole;

let data = std::fs::read("output.msi").unwrap();
let info = validate_ole(&data).unwrap();

assert!(info.valid_ole);
assert!(info.has_summary);
assert!(info.has_string_pool);
println!("Streams: {:?}", info.stream_names);
println!("Tables: {:?}", info.table_streams);
```

## OLE V3 Format

The OLE writer produces V3 compound files with:
- 512-byte sectors (required by Windows Installer)
- 64-byte mini-sectors
- Mini-stream for data < 4096 bytes
- Regular sector chains for data >= 4096 bytes
- MSI CLSID: `{000C1084-0000-0000-C000-000000000046}`

## MSI Table Serialization

Tables are serialized in **column-major order** (not row-major) as required by the MSI specification. Rows are sorted by primary key (using string pool IDs for string columns).

## Testing

```bash
cargo test -p velocity-msi
```

The test suite includes:
- 51 unit tests (OLE writer, cabinet, summary, tables, string pool, validation)
- 27 comprehensive validation tests (DIFAT pointers, mini-stream boundaries, FAT/MiniFAT chains, directory BST, cabinet round-trip, stress tests, cross-validation with msi crate)
- 9 cross-validation tests (independent verification with cfb crate)
- 1 doc test

All 88 tests pass.

## License

Licensed under either of:
- Apache License, Version 2.0
- MIT license
