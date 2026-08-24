/// Definitive test: Create MSI via msi crate (cfb OLE) vs our custom OLE writer.
///
/// Both MSIs have IDENTICAL data (same tables, same rows, same string pool codepage).
/// The ONLY difference is the OLE layer:
/// - Our MSI: custom OLE writer (ole.rs)
/// - Reference MSI: msi crate's cfb-based writer
///
/// If reference works but ours doesn't → OLE writer bug
/// If both fail → data bug
/// If both work → issue is fixed
use std::io::Cursor;
use std::process::Command;

fn main() {
    println!("=== Definitive OLE comparison test ===\n");

    // ── Build MSI via msi crate (cfb-based OLE) ────────────────────
    println!("--- Building reference MSI via msi crate ---");
    let ref_data = build_via_msi_crate();
    let ref_path = "definitive_ref.msi";
    std::fs::write(ref_path, &ref_data).unwrap();
    println!("Reference MSI: {} bytes", ref_data.len());

    // ── Build MSI via velocity-msi (custom OLE) ────────────────────
    println!("\n--- Building velocity-msi MSI ---");
    let our_data = build_via_velocity_msi();
    let our_path = "definitive_our.msi";
    std::fs::write(our_path, &our_data).unwrap();
    println!("Our MSI: {} bytes", our_data.len());

    // ── Test both with msiexec ─────────────────────────────────────
    println!("\n=== Testing with msiexec ===");
    let code_ref = test_msiexec(ref_path, "reference (msi crate + cfb)");
    let code_our = test_msiexec(our_path, "ours (custom OLE writer)");

    // ── Summary ────────────────────────────────────────────────────
    println!("\n\n=== RESULTS ===");
    println!("Reference (msi crate + cfb): exit code {}", code_ref);
    println!("Ours (custom OLE writer):    exit code {}", code_our);

    if code_ref == 0 && code_our != 0 {
        println!("\n>>> ROOT CAUSE: Custom OLE writer has a structural bug!");
        println!(">>> The data format is correct, but the OLE container is broken.");
    } else if code_ref != 0 && code_our != 0 {
        println!("\n>>> Both fail: need to investigate further.");
    } else if code_ref == 0 && code_our == 0 {
        println!("\n>>> Both work! Issue is fixed.");
    } else {
        println!("\n>>> Unexpected: ours works but reference doesn't?!");
    }
}

fn build_via_msi_crate() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor)
        .expect("Package::create should work with Cursor");

    // Set codepage to 1252 (Windows-1252) to match our MSI
    package.set_database_codepage(msi::CodePage::Windows1252);

    // Set SummaryInfo properties
    let summary = package.summary_info_mut();
    summary.set_codepage(msi::CodePage::Windows1252);
    summary.set_title("Definitive OLE Test");
    summary.set_author("Velocity");
    summary.set_arch("Intel");
    summary.set_languages(&[msi::Language::from_code(1033)]);
    summary.set_subject("Test");
    summary.set_comments("Test");
    summary.set_creating_application("Velocity Installer");
    summary.set_creation_time_to_now();
    summary.set_word_count(2);
    summary.set_uuid(uuid::Uuid::parse_str("12345678-1234-4234-8234-123456789abc").unwrap());

    // Create Property table - use standard column sizes only (max 255)
    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().string(255),
    ]).unwrap();

    package.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("OLE Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Corp".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())])
        .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}".into())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    // Create Directory table
    package.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").primary_key().string(255),
    ]).unwrap();

    package.insert_rows(msi::Insert::into("Directory")
        .row(vec![
            msi::Value::Str("TARGETDIR".into()),
            msi::Value::Null,
            msi::Value::Str("SourceDir".into()),
        ])
        .row(vec![
            msi::Value::Str("INSTALLDIR".into()),
            msi::Value::Str("TARGETDIR".into()),
            msi::Value::Str("VelTest:VelTest".into()),
        ])
    ).unwrap();

    // Create InstallExecuteSequence table
    package.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();

    package.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![
            msi::Value::Str("CostInitialize".into()),
            msi::Value::Null,
            msi::Value::Int(800),
        ])
        .row(vec![
            msi::Value::Str("CostFinalize".into()),
            msi::Value::Null,
            msi::Value::Int(1000),
        ])
    ).unwrap();

    package.flush().unwrap();

    let cursor = package.into_inner().unwrap();
    cursor.into_inner()
}

fn build_via_velocity_msi() -> Vec<u8> {
    let mut b = velocity_msi::MsiBuilder::new();
    b.set_title("Definitive OLE Test");
    b.set_author("Velocity");
    b.set_template("Intel", 1033);

    // Use standard column sizes (max 255 for strings)
    b.create_table("Property", vec![
        velocity_msi::Column::build("Property").string(72).primary_key().build(),
        velocity_msi::Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![velocity_msi::Value::from("ProductName"), velocity_msi::Value::from("OLE Test")],
        vec![velocity_msi::Value::from("ProductVersion"), velocity_msi::Value::from("1.0.0")],
        vec![velocity_msi::Value::from("Manufacturer"), velocity_msi::Value::from("Velocity Corp")],
        vec![velocity_msi::Value::from("ProductCode"), velocity_msi::Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![velocity_msi::Value::from("UpgradeCode"), velocity_msi::Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![velocity_msi::Value::from("ProductLanguage"), velocity_msi::Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        velocity_msi::Column::build("Directory").string(72).primary_key().build(),
        velocity_msi::Column::build("Directory_Parent").string(72).nullable().build(),
        velocity_msi::Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::Null, velocity_msi::Value::from("SourceDir")],
        vec![velocity_msi::Value::from("INSTALLDIR"), velocity_msi::Value::from("TARGETDIR"), velocity_msi::Value::from("VelTest:VelTest")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        velocity_msi::Column::build("Action").string(72).primary_key().build(),
        velocity_msi::Column::build("Condition").string(255).nullable().build(),
        velocity_msi::Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![velocity_msi::Value::from("CostInitialize"), velocity_msi::Value::Null, velocity_msi::Value::Int(800)],
        vec![velocity_msi::Value::from("CostFinalize"), velocity_msi::Value::Null, velocity_msi::Value::Int(1000)],
    ]).unwrap();

    b.build().unwrap()
}

fn test_msiexec(path: &str, label: &str) -> i32 {
    println!("\n--- {} ---", label);

    // Verify with msi crate
    let file = std::fs::File::open(path).unwrap();
    match msi::Package::open(file) {
        Ok(pkg) => {
            println!("  msi crate: opened OK, {} tables", pkg.tables().count());
            for t in pkg.tables() {
                print!("    {} (", t.name());
                for (i, c) in t.columns().iter().enumerate() {
                    if i > 0 { print!(", "); }
                    let type_str = match c.coltype() {
                        msi::ColumnType::Int16 => "i16".to_string(),
                        msi::ColumnType::Int32 => "i32".to_string(),
                        msi::ColumnType::Str(n) => format!("str{}", n),
                    };
                    print!("{}", type_str);
                }
                print!(")");
                println!();
            }
        }
        Err(e) => {
            println!("  msi crate: FAILED to open: {}", e);
        }
    }

    // Test with msiexec
    let output = Command::new("msiexec")
        .args(&["/i", path, "/qn", "/norestart", "/l*v", &format!("{}.log", label.replace(' ', "_"))])
        .output()
        .expect("Failed to run msiexec");
    let code = output.status.code().unwrap_or(-1);
    println!("  msiexec exit code: {}", code);

    // Try to uninstall if install succeeded
    if code == 0 {
        let _ = Command::new("msiexec")
            .args(&["/x", path, "/qn", "/norestart"])
            .output();
    }

    code
}
