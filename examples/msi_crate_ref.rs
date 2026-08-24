/// Use the msi crate to create a reference MSI and test with msiexec.
use std::io::Cursor;

fn main() {
    let mut writer = Cursor::new(Vec::new());

    // Create a new MSI package
    let mut pkg = msi::Package::create(msi::PackageType::Installer, &mut writer).unwrap();

    // Create Property table
    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").string(1024),
    ]).unwrap();

    // Insert properties
    for (prop, val) in &[
        ("ProductName", "MSI Crate Test"),
        ("ProductVersion", "1.0.0"),
        ("Manufacturer", "Velocity Corp"),
        ("ProductCode", "{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}"),
        ("UpgradeCode", "{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}"),
        ("ProductLanguage", "1033"),
    ] {
        pkg.insert_rows(msi::Insert::into("Property").row(vec![
            msi::Value::Str((*prop).into()),
            msi::Value::Str((*val).into()),
        ])).unwrap();
    }

    // Create Directory table
    pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").primary_key().string(255),
    ]).unwrap();

    pkg.insert_rows(msi::Insert::into("Directory").row(vec![
        msi::Value::Str("TARGETDIR".into()),
        msi::Value::Null,
        msi::Value::Str("SourceDir".into()),
    ])).unwrap();
    pkg.insert_rows(msi::Insert::into("Directory").row(vec![
        msi::Value::Str("INSTALLDIR".into()),
        msi::Value::Str("TARGETDIR".into()),
        msi::Value::Str("VelTest:VelTest".into()),
    ])).unwrap();

    // Create InstallExecuteSequence table
    pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();

    for (action, seq) in &[
        ("CostInitialize", 800),
        ("FileCost", 900),
        ("CostFinalize", 1000),
        ("InstallValidate", 1400),
        ("InstallInitialize", 1500),
        ("InstallFiles", 4000),
        ("InstallFinalize", 6600),
    ] {
        pkg.insert_rows(msi::Insert::into("InstallExecuteSequence").row(vec![
            msi::Value::Str((*action).into()),
            msi::Value::Null,
            msi::Value::Int(*seq),
        ])).unwrap();
    }

    // Set summary info via the msi crate's SummaryInfo API
    {
        let si = pkg.summary_info_mut();
        si.set_title("MSI Crate Test");
        si.set_author("Velocity");
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_uuid(uuid::Uuid::parse_str("12345678-1234-1234-1234-123456789ABC").unwrap());
        si.set_creating_application("Test");
        si.set_word_count(2);
    }

    // Flush to write SummaryInfo and string pool
    pkg.flush().unwrap();
    drop(pkg); // Ensure Drop runs to finalize the package

    let data = writer.into_inner();
    std::fs::write("msi_crate_ref.msi", &data).unwrap();
    println!("msi crate reference MSI: {} bytes", data.len());

    // Test with msiexec
    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "msi_crate_ref.msi", "/qn", "/l*v", "msi_crate_ref.log"])
        .output().unwrap();
    let exit = output.status.code().unwrap_or(-1);
    println!("msiexec exit: {}", exit);

    if exit != 0 {
        if let Ok(log) = std::fs::read_to_string("msi_crate_ref.log") {
            for line in log.lines() {
                if (line.contains("Error ") && !line.contains("Error 0") && !line.contains("2205") && !line.contains("2228"))
                    || line.contains("return value 3")
                {
                    println!("  {}", line.trim());
                }
            }
        }
    }
}
