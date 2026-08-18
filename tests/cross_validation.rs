//! Cross-validation tests: generate MSIs with velocity-msi, then read them back
//! with the `cfb` crate (a well-tested, independent CFB implementation) to verify
//! structural correctness.
//!
//! Also tests stream data round-trip: write known data, read it back, compare.

use std::collections::HashSet;
use std::io::{Cursor, Read};

use cfb::CompoundFile;
use velocity_msi::{Column, MsiBuilder, Value};

/// Build a minimal but complete MSI for testing.
fn build_test_msi() -> Vec<u8> {
    let mut builder = MsiBuilder::new();
    builder.set_title("Cross-Validation Test");
    builder.set_author("Velocity");
    builder.set_template("x64", 1033);

    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(255).nullable().build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Property",
            vec![
                vec![Value::from("ProductName"), Value::from("Test Product")],
                vec![Value::from("ProductVersion"), Value::from("1.0.0")],
                vec![Value::from("Manufacturer"), Value::from("Test Corp")],
            ],
        )
        .unwrap();

    builder.build().unwrap()
}

/// Build an MSI with an extra large stream (>= 4096 bytes, stored in regular sectors).
fn build_msi_with_large_stream() -> (Vec<u8>, Vec<u8>) {
    let mut builder = MsiBuilder::new();
    builder.set_title("Large Stream Test");
    builder.set_author("Velocity");

    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(255).nullable().build(),
            ],
        )
        .unwrap();

    builder
        .insert_rows(
            "Property",
            vec![vec![
                Value::from("ProductName"),
                Value::from("Large Test"),
            ]],
        )
        .unwrap();

    // Add a large stream (cabinet-sized, > 4096 bytes)
    let large_data: Vec<u8> = (0..12000).map(|i| (i % 251) as u8).collect(); // prime modulus for varied data
    builder.add_stream("test.cab".to_string(), large_data.clone());

    let msi_data = builder.build().unwrap();
    (msi_data, large_data)
}

// ── cfb cross-validation tests ──────────────────────────────────────────

#[test]
fn test_cfb_can_open_generated_msi() {
    let msi_data = build_test_msi();
    let cursor = Cursor::new(&msi_data);
    let comp = CompoundFile::open(cursor).expect("cfb should be able to open our MSI");

    // Verify version
    assert_eq!(
        comp.version(),
        cfb::Version::V4,
        "cfb should detect V4 format"
    );
}

#[test]
fn test_cfb_root_entry_has_msi_clsid() {
    let msi_data = build_test_msi();
    let cursor = Cursor::new(&msi_data);
    let comp = CompoundFile::open(cursor).unwrap();

    let root = comp.root_entry();
    assert!(root.is_root(), "Root entry should be root");
    assert!(root.is_storage(), "Root entry should be storage");

    // MSI CLSID: {000C1084-0000-0000-C000-000000000046}
    let expected_clsid =
        uuid::Uuid::parse_str("000C1084-0000-0000-C000-000000000046").unwrap();
    assert_eq!(
        *root.clsid(),
        expected_clsid,
        "Root entry CLSID should be MSI CLSID"
    );
}

#[test]
fn test_cfb_entry_count_and_types() {
    let msi_data = build_test_msi();
    let cursor = Cursor::new(&msi_data);
    let comp = CompoundFile::open(cursor).unwrap();

    let entries: Vec<_> = comp.walk().collect();

    // Should have: root + SummaryInfo + 2 string pool + 3 system tables + 1 user table = 8
    // (Property table + _Tables + _Columns + _Validation)
    let stream_entries: Vec<_> = entries.iter().filter(|e| e.is_stream()).collect();
    let storage_entries: Vec<_> = entries.iter().filter(|e| e.is_storage()).collect();

    assert_eq!(
        storage_entries.len(),
        1,
        "Should have exactly 1 storage (root)"
    );
    assert!(
        stream_entries.len() >= 6,
        "Should have at least 6 streams (SummaryInfo + 2 string pool + 3 system tables + 1 user table), got {}",
        stream_entries.len()
    );
}

#[test]
fn test_cfb_all_streams_have_correct_sizes() {
    let msi_data = build_test_msi();
    let cursor = Cursor::new(&msi_data);
    let comp = CompoundFile::open(cursor).unwrap();

    for entry in comp.walk() {
        if entry.is_stream() {
            // All streams should have non-negative size
            let len = entry.len();
            assert!(
                len < msi_data.len() as u64,
                "Stream '{}' size {} should be less than total file size {}",
                entry.name(),
                len,
                msi_data.len()
            );
        }
    }
}

#[test]
fn test_cfb_summary_info_stream_exists() {
    let msi_data = build_test_msi();
    let cursor = Cursor::new(&msi_data);
    let mut comp = CompoundFile::open(cursor).unwrap();

    // Find the SummaryInformation stream
    let summary_entry = comp
        .walk()
        .find(|e| e.name().contains("SummaryInformation"));

    assert!(
        summary_entry.is_some(),
        "Should have a SummaryInformation stream"
    );

    let entry = summary_entry.unwrap();
    assert!(entry.is_stream());
    assert!(entry.len() > 0, "SummaryInformation should have content");

    // Read the SummaryInformation data via cfb
    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();

    // Verify Property Set header: BOM = 0xFFFE
    assert!(data.len() >= 48, "SummaryInformation should be at least 48 bytes");
    assert_eq!(
        u16::from_le_bytes([data[0], data[1]]),
        0xFFFE,
        "Property Set BOM should be 0xFFFE"
    );
}

#[test]
fn test_cfb_large_stream_round_trip() {
    let (msi_data, expected_data) = build_msi_with_large_stream();
    let cursor = Cursor::new(&msi_data);
    let mut comp = CompoundFile::open(cursor).unwrap();

    // Find the "test.cab" stream
    let cab_entry = comp
        .walk()
        .find(|e| e.name() == "test.cab");

    assert!(
        cab_entry.is_some(),
        "Should find the test.cab stream"
    );

    let entry = cab_entry.unwrap();
    assert_eq!(
        entry.len(),
        expected_data.len() as u64,
        "Stream size should match original data"
    );

    // Read back the stream data via cfb and compare
    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut actual_data = Vec::new();
    stream.read_to_end(&mut actual_data).unwrap();

    assert_eq!(
        actual_data.len(),
        expected_data.len(),
        "Read-back data length should match"
    );
    assert_eq!(
        actual_data, expected_data,
        "Read-back data should match original"
    );
}

#[test]
fn test_cfb_small_stream_round_trip() {
    // Build an MSI with a small extra stream (< 4096 bytes, stored in mini-stream)
    let mut builder = MsiBuilder::new();
    builder.set_title("Small Stream Test");
    builder.set_author("Velocity");

    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(255).nullable().build(),
            ],
        )
        .unwrap();
    builder
        .insert_rows(
            "Property",
            vec![vec![Value::from("Test"), Value::from("Value")]],
        )
        .unwrap();

    let small_data = vec![0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE];
    builder.add_stream("marker.dat".to_string(), small_data.clone());

    let msi_data = builder.build().unwrap();
    let cursor = Cursor::new(&msi_data);
    let mut comp = CompoundFile::open(cursor).unwrap();

    // Find and read back the small stream
    let entry = comp
        .walk()
        .find(|e| e.name() == "marker.dat")
        .expect("Should find marker.dat stream");

    assert_eq!(entry.len(), 6, "Stream size should be 6 bytes");

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut actual = Vec::new();
    stream.read_to_end(&mut actual).unwrap();

    assert_eq!(actual, small_data, "Mini-stream data should round-trip exactly");
}

#[test]
fn test_cfb_stream_names_are_unique() {
    let msi_data = build_test_msi();
    let cursor = Cursor::new(&msi_data);
    let comp = CompoundFile::open(cursor).unwrap();

    let names: HashSet<String> = comp
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    let total_streams = comp.walk().filter(|e| e.is_stream()).count();
    assert_eq!(
        names.len(),
        total_streams,
        "All stream names should be unique"
    );
}

#[test]
fn test_cfb_multiple_extra_streams_round_trip() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Multi-Stream Test");
    builder.set_author("Velocity");

    builder
        .create_table(
            "Property",
            vec![
                Column::build("Property").string(72).primary_key().build(),
                Column::build("Value").string(255).nullable().build(),
            ],
        )
        .unwrap();
    builder
        .insert_rows(
            "Property",
            vec![vec![Value::from("A"), Value::from("B")]],
        )
        .unwrap();

    // Add multiple streams of varying sizes
    let streams_data = vec![
        ("tiny.bin", vec![42u8]),
        ("small.bin", vec![0xAB; 200]),
        ("medium.bin", vec![0xCD; 3000]),
        ("large.bin", (0..8192).map(|i| (i % 127) as u8).collect()),
    ];

    for (name, data) in &streams_data {
        builder.add_stream(name.to_string(), data.clone());
    }

    let msi_data = builder.build().unwrap();
    let cursor = Cursor::new(&msi_data);
    let mut comp = CompoundFile::open(cursor).unwrap();

    // Verify each stream round-trips correctly
    for (name, expected) in &streams_data {
        let entry = comp
            .walk()
            .find(|e| e.name() == *name)
            .unwrap_or_else(|| panic!("Should find stream '{}'", name));

        assert_eq!(
            entry.len(),
            expected.len() as u64,
            "Stream '{}' size mismatch",
            name
        );

        let path = entry.path().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&path).unwrap();
        let mut actual = Vec::new();
        stream.read_to_end(&mut actual).unwrap();

        assert_eq!(
            actual.len(),
            expected.len(),
            "Stream '{}' data length mismatch",
            name
        );
        assert_eq!(actual, *expected, "Stream '{}' data mismatch", name);
    }
}
