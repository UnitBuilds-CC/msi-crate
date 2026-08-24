/// Test: Does msi crate's Package::create() produce a working MSI with
/// Property + Directory + InstallExecuteSequence?
fn main() {
    println!("=== msi crate create() test ===\n");

    let path = "ref_msi_create_test.msi";
    let _ = std::fs::remove_file(path);
    match create_msi(path) {
        Ok(()) => {
            let size = std::fs::metadata(path).unwrap().len();
            println!("Created: {} ({} bytes)\n", path, size);
            test_msi(path, "ref_create");
        }
        Err(e) => {
            println!("FAILED to create: {:?}", e);
            // Try to get more details
            println!("Trying direct file create...");
            match std::fs::File::create("test_write.tmp") {
                Ok(f) => {
                    println!("Direct file create works!");
                    drop(f);
                    let _ = std::fs::remove_file("test_write.tmp");
                }
                Err(e2) => println!("Direct file create also fails: {:?}", e2),
            }
        }
    }
}

fn create_msi(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = std::fs::File::create(path)?;
    let mut package = msi::Package::create(msi::PackageType::Installer, file)?;

    // Property table
    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").string(1024),
    ])?;
    package.insert_rows(
        msi::Insert::into("Property")
            .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("MSI Ref Test".into())])
            .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
            .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Velocity Corp".into())])
            .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())])
            .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}".into())])
            .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    )?;

    // Directory table
    package.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").primary_key().string(255),
    ])?;
    package.insert_rows(
        msi::Insert::into("Directory")
            .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
            .row(vec![msi::Value::Str("INSTALLDIR".into()), msi::Value::Str("TARGETDIR".into()), msi::Value::Str("VelTest".into())])
    )?;

    // InstallExecuteSequence
    package.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ])?;
    package.insert_rows(
        msi::Insert::into("InstallExecuteSequence")
            .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
            .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
    )?;

    // Set summary info
    {
        let summary = package.summary_info_mut();
        summary.set_title("MSI Ref Test");
        summary.set_subject("Testing");
        summary.set_author("Velocity Corp");
        summary.set_comments("Test");
        summary.set_arch("Intel");
        summary.set_languages(&[msi::Language::from_code(1033)]);
        summary.set_creating_application("Velocity Installer");
        summary.set_word_count(2);
        summary.set_creation_time_to_now();
        summary.set_uuid(uuid::Uuid::parse_str("F29F85E0-4FF9-1068-AB91-08002B27B3D9")?);
    }

    package.flush()?;
    Ok(())
}

fn test_msi(path: &str, label: &str) {
    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", path, "/qn", "/l*v", &format!("{}.log", label)])
        .output().unwrap();
    let exit = output.status.code().unwrap_or(-1);
    println!("msiexec exit={}", exit);
    if exit != 0 {
        if let Ok(log) = std::fs::read_to_string(format!("{}.log", label)) {
            for line in log.lines() {
                if (line.contains("Error ") && !line.contains("Error 0") && !line.contains("2205") && !line.contains("2228"))
                    || line.contains("return value 3")
                {
                    println!("  {}", line.trim());
                }
            }
        }
    } else {
        println!("SUCCESS! msi crate create() produces installable MSI!");
    }
}
