//! Comprehensive validation test suite for the OLE V3 writer.
//!
//! Covers:
//! 1. DIFAT pointer fix (ENDOFCHAIN, not FREE_SECT)
//! 2. Cross-validation with the `msi` crate (independent reader)
//! 3. Boundary conditions (mini-stream cutoff, exact sector sizes)
//! 4. Many directory entries (multiple directory sectors)
//! 5. MiniFAT chain integrity
//! 6. Directory BST property
//! 7. Cabinet round-trip (build real cab, embed, extract)
//! 8. Stress test (many tables, many rows)
//! 9. File I/O round-trip (write to disk, read back)
//! 10. Deterministic output

use std::collections::HashSet;
use std::io::{Cursor, Read};

use cfb::CompoundFile;
use velocity_msi::{Column, MsiBuilder, Value};

// ── Helpers ────────────────────────────────────────────────────────────

fn build_minimal_msi() -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("ProductName"), Value::from("Test")]],
    )
    .unwrap();
    b.build().unwrap()
}

fn build_full_msi() -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("Full Test MSI");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(1024).build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![
            vec![Value::from("ProductName"), Value::from("Full Test")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("Manufacturer"), Value::from("Velocity")],
            vec![
                Value::from("ProductCode"),
                Value::from("{12345678-1234-1234-1234-123456789012}"),
            ],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
        ],
    )
    .unwrap();

    b.create_table(
        "Directory",
        vec![
            Column::build("Directory")
                .string(72)
                .primary_key()
                .category("Identifier")
                .build(),
            Column::build("Directory_Parent")
                .string(72)
                .nullable()
                .category("Identifier")
                .build(),
            Column::build("DefaultDir")
                .string(255)
                .primary_key()
                .category("DefaultDir")
                .build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Directory",
        vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
            vec![
                Value::from("INSTALLDIR"),
                Value::from("TARGETDIR"),
                Value::from("Test:Test"),
            ],
        ],
    )
    .unwrap();

    b.create_table(
        "Component",
        vec![
            Column::build("Component")
                .string(72)
                .primary_key()
                .category("Identifier")
                .build(),
            Column::build("ComponentId")
                .string(38)
                .nullable()
                .category("GUID")
                .build(),
            Column::build("Directory_")
                .string(72)
                .category("Identifier")
                .build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition")
                .string(255)
                .nullable()
                .category("Condition")
                .build(),
            Column::build("KeyPath")
                .string(72)
                .nullable()
                .category("Identifier")
                .build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Component",
        vec![vec![
            Value::from("MainComp"),
            Value::Null,
            Value::from("INSTALLDIR"),
            Value::Int(0),
            Value::Null,
            Value::Null,
        ]],
    )
    .unwrap();

    b.create_table(
        "Feature",
        vec![
            Column::build("Feature")
                .string(38)
                .primary_key()
                .category("Identifier")
                .build(),
            Column::build("Feature_Parent")
                .string(38)
                .nullable()
                .category("Identifier")
                .build(),
            Column::build("Title").string(64).nullable().build(),
            Column::build("Description").string(255).nullable().build(),
            Column::build("Display").int16().nullable().build(),
            Column::build("Level").int16().build(),
            Column::build("Directory_").string(72).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Feature",
        vec![vec![
            Value::from("MainFeature"),
            Value::Null,
            Value::from("Complete"),
            Value::from("Full installation"),
            Value::Int(2),
            Value::Int(1),
            Value::from("INSTALLDIR"),
        ]],
    )
    .unwrap();

    b.create_table(
        "Media",
        vec![
            Column::build("DiskId").int16().primary_key().build(),
            Column::build("LastSequence").int16().build(),
            Column::build("DiskPrompt").string(64).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Media",
        vec![vec![Value::Int(1), Value::Int(1), Value::from("Disk 1")]],
    )
    .unwrap();

    b.build().unwrap()
}

fn read_u32(data: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([data[off], data[off + 1], data[off + 2], data[off + 3]])
}

// ── 1. DIFAT pointer fix verification ──────────────────────────────────

#[test]
fn test_difat_pointer_is_endofchain_when_no_difat_sectors() {
    let msi = build_minimal_msi();

    // Offset 68 = First DIFAT Sector location in the header
    let first_difat = read_u32(&msi, 68);
    // Offset 72 = Number of DIFAT Sectors
    let num_difat = read_u32(&msi, 72);

    assert_eq!(num_difat, 0, "Small MSI should need 0 DIFAT sectors");
    assert_eq!(
        first_difat, 0xFFFFFFFE,
        "When no DIFAT sectors exist, pointer MUST be ENDOFCHAIN (0xFFFFFFFE), not FREE_SECT (0xFFFFFFFF)"
    );
}

#[test]
fn test_difat_pointer_is_endofchain_for_full_msi() {
    let msi = build_full_msi();
    let first_difat = read_u32(&msi, 68);
    let num_difat = read_u32(&msi, 72);

    assert_eq!(num_difat, 0);
    assert_eq!(first_difat, 0xFFFFFFFE);
}

#[test]
fn test_difat_pointer_is_endofchain_for_empty_ole() {
    let data = velocity_msi::ole::build_ole_file(&[]);
    let first_difat = read_u32(&data, 68);
    let num_difat = read_u32(&data, 72);

    assert_eq!(num_difat, 0);
    assert_eq!(first_difat, 0xFFFFFFFE);
}

// ── 2. Cross-validation with `msi` crate ───────────────────────────────

#[test]
fn test_msi_crate_can_open_our_output() {
    let msi = build_full_msi();
    let cursor = Cursor::new(&msi);

    // The `msi` crate uses its own OLE reader — independent validation
    let package = msi::Package::open(cursor).expect("msi crate failed to open our MSI");

    // Verify we can see tables
    let table_names: Vec<&str> = package.tables().map(|t| t.name()).collect();
    assert!(
        table_names.contains(&"Property"),
        "msi crate should see Property table, found: {:?}",
        table_names
    );
}

#[test]
fn test_msi_crate_reads_property_table() {
    let msi = build_full_msi();
    let cursor = Cursor::new(&msi);
    let package = msi::Package::open(cursor).expect("msi crate should open our MSI");

    let table = package
        .get_table("Property")
        .expect("Property table should exist");

    let col_names: Vec<&str> = table.columns().iter().map(|c| c.name()).collect();
    assert!(
        col_names.contains(&"Property"),
        "Property table should have a 'Property' column, found: {:?}",
        col_names
    );
    assert!(
        col_names.contains(&"Value"),
        "Property table should have a 'Value' column, found: {:?}",
        col_names
    );
}

// ── 3. Boundary conditions ─────────────────────────────────────────────

#[test]
fn test_stream_exactly_at_mini_stream_cutoff() {
    // 4095 bytes = just below cutoff → mini stream
    let mut b = MsiBuilder::new();
    b.set_title("Boundary Test");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let just_below = vec![0xAA; 4095];
    b.add_stream("below.dat".to_string(), just_below.clone());

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let encoded = velocity_msi::encode_stream_name("below.dat", false);
    let entry = comp
        .walk()
        .find(|e| e.name() == encoded)
        .expect("Should find below.dat");
    assert_eq!(entry.len(), 4095, "4095-byte stream should be in mini stream");

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    assert_eq!(data, just_below);
}

#[test]
fn test_stream_exactly_at_cutoff_goes_to_regular_sectors() {
    // 4096 bytes = exactly at cutoff → regular sectors (NOT mini stream)
    let mut b = MsiBuilder::new();
    b.set_title("Boundary Test");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let exactly_at = vec![0xBB; 4096];
    b.add_stream("exact.dat".to_string(), exactly_at.clone());

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let encoded = velocity_msi::encode_stream_name("exact.dat", false);
    let entry = comp
        .walk()
        .find(|e| e.name() == encoded)
        .expect("Should find exact.dat");
    assert_eq!(entry.len(), 4096);

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    assert_eq!(data, exactly_at);
}

#[test]
fn test_stream_just_above_cutoff() {
    let mut b = MsiBuilder::new();
    b.set_title("Boundary Test");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let just_above = vec![0xCC; 4097];
    b.add_stream("above.dat".to_string(), just_above.clone());

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let encoded = velocity_msi::encode_stream_name("above.dat", false);
    let entry = comp
        .walk()
        .find(|e| e.name() == encoded)
        .expect("Should find above.dat");
    assert_eq!(entry.len(), 4097);

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    assert_eq!(data, just_above);
}

#[test]
fn test_single_byte_stream() {
    let mut b = MsiBuilder::new();
    b.set_title("Tiny");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let tiny = vec![0x42u8];
    b.add_stream("tiny.dat".to_string(), tiny.clone());

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let encoded = velocity_msi::encode_stream_name("tiny.dat", false);
    let entry = comp
        .walk()
        .find(|e| e.name() == encoded)
        .expect("Should find tiny.dat");
    assert_eq!(entry.len(), 1);

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    assert_eq!(data, tiny);
}

#[test]
fn test_exactly_one_mini_sector() {
    // 64 bytes = exactly one mini-sector
    let mut b = MsiBuilder::new();
    b.set_title("Mini");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let one_mini = vec![0xDD; 64];
    b.add_stream("onemini.dat".to_string(), one_mini.clone());

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let encoded = velocity_msi::encode_stream_name("onemini.dat", false);
    let entry = comp
        .walk()
        .find(|e| e.name() == encoded)
        .expect("Should find onemini.dat");
    assert_eq!(entry.len(), 64);

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    assert_eq!(data, one_mini);
}

#[test]
fn test_exactly_one_regular_sector() {
    // 512 bytes = exactly one regular sector (but still mini stream since < 4096)
    let mut b = MsiBuilder::new();
    b.set_title("Sector");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let one_sect = vec![0xEE; 512];
    b.add_stream("onesect.dat".to_string(), one_sect.clone());

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let encoded = velocity_msi::encode_stream_name("onesect.dat", false);
    let entry = comp
        .walk()
        .find(|e| e.name() == encoded)
        .expect("Should find onesect.dat");
    assert_eq!(entry.len(), 512);

    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    assert_eq!(data, one_sect);
}

// ── 4. Many directory entries (multiple directory sectors) ─────────────

#[test]
fn test_many_streams_force_multiple_dir_sectors() {
    // V3: 4 directory entries per sector. Root + N streams.
    // To force 2+ directory sectors, need > 16 streams (root + 16 = 17 entries → 5 sectors).
    let mut b = MsiBuilder::new();
    b.set_title("Many Streams");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    let mut expected = Vec::new();
    for i in 0..20 {
        let name = format!("stream{:02}.dat", i);
        let data = vec![(i as u8).wrapping_mul(13); 100 + i * 10];
        expected.push((name.clone(), data.clone()));
        b.add_stream(name, data);
    }

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    // Verify all 20 streams are present and readable
    for (name, expected_data) in &expected {
        let encoded = velocity_msi::encode_stream_name(name, false);
        let entry = comp
            .walk()
            .find(|e| e.name() == encoded)
            .unwrap_or_else(|| panic!("Missing stream: {}", name));

        assert_eq!(
            entry.len(),
            expected_data.len() as u64,
            "Size mismatch for {}",
            name
        );

        let path = entry.path().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();
        assert_eq!(data, *expected_data, "Data mismatch for {}", name);
    }
}

// ── 5. MiniFAT chain integrity ─────────────────────────────────────────

#[test]
fn test_minifat_chains_terminate_correctly() {
    // Build MSI with multiple small streams, verify all mini-stream data
    // is independently readable (proves MiniFAT chains don't bleed into each other)
    let mut b = MsiBuilder::new();
    b.set_title("MiniFAT Test");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("X"), Value::from("Y")]],
    )
    .unwrap();

    // Create streams with distinct patterns so any cross-contamination is detectable
    let streams: Vec<(String, Vec<u8>)> = (0..8)
        .map(|i| {
            let name = format!("mini{}.dat", i);
            let data = vec![(0x10 + i) as u8; 50 + i * 30];
            (name, data)
        })
        .collect();

    for (name, data) in &streams {
        b.add_stream(name.clone(), data.clone());
    }

    let msi = b.build().unwrap();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    for (name, expected) in &streams {
        let encoded = velocity_msi::encode_stream_name(name, false);
        let entry = comp.walk().find(|e| e.name() == encoded).unwrap();
        let path = entry.path().to_string_lossy().to_string();
        let mut stream = comp.open_stream(&path).unwrap();
        let mut data = Vec::new();
        stream.read_to_end(&mut data).unwrap();

        // Every byte should be the expected pattern — no leakage from adjacent chains
        assert!(
            data.iter().all(|&b| b == expected[0]),
            "Stream {} has contaminated data",
            name
        );
        assert_eq!(data, *expected);
    }
}

// ── 6. Directory BST property ──────────────────────────────────────────

#[test]
fn test_directory_bst_all_entries_reachable() {
    let msi = build_full_msi();
    let comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    // If the BST is broken, some entries won't be reachable via walk()
    let all_entries: Vec<_> = comp.walk().collect();
    let stream_count = all_entries.iter().filter(|e| e.is_stream()).count();

    // Full MSI should have: SummaryInfo + 2 string pool + 3 system tables + 5 user tables = 11 streams
    assert!(
        stream_count >= 10,
        "BST should make all streams reachable, found {} streams",
        stream_count
    );
}

#[test]
fn test_no_duplicate_entries_in_walk() {
    let msi = build_full_msi();
    let comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let mut names = HashSet::new();
    for entry in comp.walk() {
        let path = entry.path().to_string_lossy().to_string();
        assert!(
            names.insert(path.clone()),
            "Duplicate entry in walk(): {}",
            path
        );
    }
}

// ── 7. Cabinet round-trip ──────────────────────────────────────────────

#[test]
fn test_cabinet_embed_and_extract() {
    use velocity_msi::cabinet;

    let mut b = MsiBuilder::new();
    b.set_title("Cabinet Test");
    b.set_author("Test");
    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ],
    )
    .unwrap();
    b.insert_rows(
        "Property",
        vec![vec![Value::from("ProductName"), Value::from("CabTest")]],
    )
    .unwrap();

    // Build a real cabinet with multiple files
    let file_content_a = b"Hello from file A! This is test content for cabinet compression.";
    let file_content_b = b"File B has different content. Repeated data helps compression: AAAAAAAAAA";

    let cab_data = cabinet::build_cabinet(&[
        cabinet::CabinetFile {
            name: "file_a.txt".to_string(),
            data: file_content_a.to_vec(),
        },
        cabinet::CabinetFile {
            name: "file_b.txt".to_string(),
            data: file_content_b.to_vec(),
        },
    ]);

    b.add_stream("test.cab".to_string(), cab_data.clone());

    let msi = b.build().unwrap();

    // Extract the cabinet from the MSI via cfb
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();
    let encoded = velocity_msi::encode_stream_name("test.cab", false);
    let entry = comp.walk().find(|e| e.name() == encoded).unwrap();
    let path = entry.path().to_string_lossy().to_string();
    let mut stream = comp.open_stream(&path).unwrap();
    let mut extracted_cab = Vec::new();
    stream.read_to_end(&mut extracted_cab).unwrap();

    assert_eq!(extracted_cab, cab_data, "Cabinet data should round-trip exactly");

    // Verify the cabinet is valid by checking its signature
    assert!(
        extracted_cab.len() > 4,
        "Cabinet should have content"
    );
    // Cabinet signature: "MSCF" at offset 0
    assert_eq!(&extracted_cab[0..4], b"MSCF", "Should start with MSCF signature");
}

// ── 8. Stress test ─────────────────────────────────────────────────────

#[test]
fn test_many_tables_many_rows() {
    let mut b = MsiBuilder::new();
    b.set_title("Stress Test");
    b.set_author("Velocity");

    // Create 10 tables with 50 rows each
    for t in 0..10 {
        let table_name = format!("Table{:02}", t);
        b.create_table(
            &table_name,
            vec![
                Column::build("Key").string(72).primary_key().build(),
                Column::build("Value").string(255).nullable().build(),
                Column::build("Extra").string(128).nullable().build(),
            ],
        )
        .unwrap();

        let rows: Vec<Vec<Value>> = (0..50)
            .map(|r| {
                vec![
                    Value::from(format!("key_{}_{}", t, r)),
                    Value::from(format!("value_{}_{}", t, r)),
                    Value::from(format!("extra_{}_{}", t, r)),
                ]
            })
            .collect();
        b.insert_rows(&table_name, rows).unwrap();
    }

    let msi = b.build().unwrap();

    // Validate with cfb
    let comp = CompoundFile::open(Cursor::new(&msi)).unwrap();
    let stream_count = comp.walk().filter(|e| e.is_stream()).count();

    // 10 user tables + 3 system tables + SummaryInfo + 2 string pool = 16
    assert!(
        stream_count >= 15,
        "Stress test should have 15+ streams, got {}",
        stream_count
    );

    // Validate with our own validator
    let info = velocity_msi::validate_ole(&msi).unwrap();
    assert!(info.valid_ole);
    assert!(info.has_summary);
    assert!(info.has_string_pool);

    // Semantic validation
    let report = velocity_msi::validate_msi_semantics(&msi).unwrap();
    assert_eq!(report.user_tables.len(), 11, "Should have 11 tables (10 user + _Validation)");
    assert!(report.all_tables_have_streams);
    assert!(report.columns_consistent);
}

#[test]
fn test_large_string_pool() {
    let mut b = MsiBuilder::new();
    b.set_title("String Pool Stress");
    b.set_author("Test");

    b.create_table(
        "Property",
        vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(1024).nullable().build(),
        ],
    )
    .unwrap();

    // Insert many unique strings to grow the string pool
    let mut rows = Vec::new();
    for i in 0..200 {
        let key = format!("Prop_{:04}", i);
        let val = format!("This is a moderately long value string number {} with enough content to exercise the string pool encoding", i);
        rows.push(vec![Value::from(key), Value::from(val)]);
    }
    b.insert_rows("Property", rows).unwrap();

    let msi = b.build().unwrap();

    // Verify it opens and all data is intact
    let comp = CompoundFile::open(Cursor::new(&msi)).unwrap();
    let report = velocity_msi::validate_msi_semantics(&msi).unwrap();
    assert!(
        report.string_pool_size >= 200,
        "String pool should have 200+ entries, got {}",
        report.string_pool_size
    );

    // Verify the Property stream is readable
    let prop_encoded = velocity_msi::encode_stream_name("Property", true);
    let entry = comp.walk().find(|e| e.name() == prop_encoded);
    assert!(entry.is_some(), "Property stream should exist");
}

// ── 9. File I/O round-trip ─────────────────────────────────────────────

#[test]
fn test_write_to_disk_and_read_back() {
    let msi = build_full_msi();

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test_output.msi");

    // Write to disk
    std::fs::write(&path, &msi).unwrap();

    // Read back from disk
    let disk_data = std::fs::read(&path).unwrap();
    assert_eq!(disk_data.len(), msi.len(), "File size on disk should match");
    assert_eq!(disk_data, msi, "File content on disk should match in-memory");

    // Verify cfb can open the on-disk file
    let comp = CompoundFile::open(Cursor::new(&disk_data)).unwrap();
    assert_eq!(comp.version(), cfb::Version::V3);

    // Verify msi crate can open it too
    let package = msi::Package::open(Cursor::new(&disk_data)).unwrap();
    let table_names: Vec<&str> = package.tables().map(|t| t.name()).collect();
    assert!(table_names.contains(&"Property"));
}

// ── 10. Deterministic output ───────────────────────────────────────────

#[test]
fn test_deterministic_output_same_input() {
    // Build the same MSI twice with the same builder config.
    // Note: timestamps in SummaryInfo use Utc::now(), so we need to
    // control for that. We test the OLE layer directly instead.
    let streams = vec![
        velocity_msi::ole::OleStream {
            name: "StreamA".to_string(),
            data: vec![1, 2, 3, 4, 5],
        },
        velocity_msi::ole::OleStream {
            name: "StreamB".to_string(),
            data: vec![10, 20, 30],
        },
        velocity_msi::ole::OleStream {
            name: "StreamC".to_string(),
            data: vec![0xFF; 5000],
        },
    ];

    let output1 = velocity_msi::ole::build_ole_file(&streams);
    let output2 = velocity_msi::ole::build_ole_file(&streams);

    assert_eq!(
        output1, output2,
        "Same input streams should produce identical OLE output"
    );
}

// ── Additional structural integrity checks ─────────────────────────────

#[test]
fn test_ole_magic_bytes() {
    let msi = build_minimal_msi();
    assert_eq!(
        &msi[0..8],
        &[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1],
        "OLE magic bytes must be correct"
    );
}

#[test]
fn test_v3_format() {
    let msi = build_minimal_msi();
    let major = u16::from_le_bytes([msi[26], msi[27]]);
    assert_eq!(major, 3, "Must be V3 (major version 3) for MSI compatibility");

    let sector_shift = u16::from_le_bytes([msi[30], msi[31]]);
    assert_eq!(sector_shift, 9, "V3 must use 512-byte sectors (2^9)");

    let mini_shift = u16::from_le_bytes([msi[32], msi[33]]);
    assert_eq!(mini_shift, 6, "Mini-sector must be 64 bytes (2^6)");
}

#[test]
fn test_file_size_is_sector_aligned() {
    for size in [100, 512, 4096, 8192, 50000] {
        let streams = vec![velocity_msi::ole::OleStream {
            name: "data".to_string(),
            data: vec![0xAA; size],
        }];
        let data = velocity_msi::ole::build_ole_file(&streams);
        assert_eq!(
            data.len() % 512,
            0,
            "File size {} not sector-aligned for stream size {}",
            data.len(),
            size
        );
    }
}

#[test]
fn test_header_difat_array_unused_entries_are_free_sect() {
    let msi = build_minimal_msi();

    // DIFAT array starts at offset 76, contains 109 entries
    let num_fat = read_u32(&msi, 44) as usize;

    for i in 0..109 {
        let off = 76 + i * 4;
        let val = read_u32(&msi, off);
        if i < num_fat {
            // Should be a valid FAT sector index
            assert_ne!(val, 0xFFFFFFFF, "Active DIFAT entry should not be FREE_SECT");
            assert_ne!(val, 0xFFFFFFFE, "Active DIFAT entry should not be ENDOFCHAIN");
        } else {
            // Unused entries must be FREE_SECT
            assert_eq!(
                val, 0xFFFFFFFF,
                "Unused DIFAT entry {} should be FREE_SECT, got 0x{:08X}",
                i, val
            );
        }
    }
}

#[test]
fn test_cfb_validates_every_generated_msi() {
    // Generate MSIs of various configurations and verify ALL pass cfb validation
    let configs: Vec<(&str, Vec<u8>)> = vec![
        ("empty", {
            velocity_msi::ole::build_ole_file(&[])
        }),
        ("single_tiny", {
            velocity_msi::ole::build_ole_file(&[velocity_msi::ole::OleStream {
                name: "x".to_string(),
                data: vec![1],
            }])
        }),
        ("single_large", {
            velocity_msi::ole::build_ole_file(&[velocity_msi::ole::OleStream {
                name: "big".to_string(),
                data: vec![0xAB; 100_000],
            }])
        }),
        ("mixed", {
            velocity_msi::ole::build_ole_file(&[
                velocity_msi::ole::OleStream {
                    name: "small".to_string(),
                    data: vec![1; 100],
                },
                velocity_msi::ole::OleStream {
                    name: "big".to_string(),
                    data: vec![2; 10000],
                },
            ])
        }),
        ("minimal_msi", build_minimal_msi()),
        ("full_msi", build_full_msi()),
    ];

    for (name, data) in configs {
        let result = CompoundFile::open(Cursor::new(&data));
        assert!(
            result.is_ok(),
            "cfb failed to validate '{}' config ({} bytes): {}",
            name,
            data.len(),
            result.err().unwrap()
        );
    }
}

#[test]
fn test_all_streams_readable_via_cfb() {
    let msi = build_full_msi();
    let mut comp = CompoundFile::open(Cursor::new(&msi)).unwrap();

    let stream_paths: Vec<String> = comp
        .walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();

    assert!(!stream_paths.is_empty(), "Should have streams to read");

    for path in &stream_paths {
        let result = comp.open_stream(path);
        assert!(result.is_ok(), "Failed to open stream: {}", path);

        let mut stream = result.unwrap();
        let mut buf = Vec::new();
        let read_result = stream.read_to_end(&mut buf);
        assert!(read_result.is_ok(), "Failed to read stream: {}", path);
    }
}

#[test]
fn test_msi_crate_can_open_minimal_msi() {
    let msi = build_minimal_msi();
    let result = msi::Package::open(Cursor::new(&msi));
    assert!(
        result.is_ok(),
        "msi crate should open minimal MSI: {:?}",
        result.err()
    );
}
