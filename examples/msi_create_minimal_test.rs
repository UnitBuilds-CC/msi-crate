/// Minimal test: Can the msi crate create a valid MSI at all?
/// Tests progressively more complex MSIs to find what breaks.
use std::io::Cursor;
use std::process::Command;

fn main() {
    println!("=== Minimal msi crate create test ===\n");

    // Test 1: Absolute minimum - just create and flush
    test_msi("test1_empty", build_empty());

    // Test 2: Add just Property table
    test_msi("test2_prop", build_with_property());

    // Test 3: Add Property + Directory
    test_msi("test3_dir", build_with_directory());

    // Test 4: Add Property + Directory + InstallExecuteSequence
    test_msi("test4_seq", build_with_sequence());
}

fn build_empty() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    package.flush().unwrap();
    package.into_inner().unwrap().into_inner()
}

fn build_with_property() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    package.set_database_codepage(msi::CodePage::Windows1252);
    let s = package.summary_info_mut();
    s.set_title("Test");
    s.set_author("Test");

    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().string(255),
    ]).unwrap();
    package.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test Product".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".into())])
    ).unwrap();
    package.flush().unwrap();
    package.into_inner().unwrap().into_inner()
}

fn build_with_directory() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    package.set_database_codepage(msi::CodePage::Windows1252);
    let s = package.summary_info_mut();
    s.set_title("Test");
    s.set_author("Test");
    s.set_arch("Intel");
    s.set_languages(&[msi::Language::from_code(1033)]);

    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().string(255),
    ]).unwrap();
    package.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test Product".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".into())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    package.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").primary_key().string(255),
    ]).unwrap();
    package.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
    ).unwrap();
    package.flush().unwrap();
    package.into_inner().unwrap().into_inner()
}

fn build_with_sequence() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut package = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    package.set_database_codepage(msi::CodePage::Windows1252);
    let s = package.summary_info_mut();
    s.set_title("Test");
    s.set_author("Test");
    s.set_arch("Intel");
    s.set_languages(&[msi::Language::from_code(1033)]);

    package.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().string(255),
    ]).unwrap();
    package.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test Product".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{12345678-1234-1234-1234-123456789012}".into())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).unwrap();

    package.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").primary_key().string(255),
    ]).unwrap();
    package.insert_rows(msi::Insert::into("Directory")
        .row(vec![msi::Value::Str("TARGETDIR".into()), msi::Value::Null, msi::Value::Str("SourceDir".into())])
    ).unwrap();

    package.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    package.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![msi::Value::Str("CostInitialize".into()), msi::Value::Null, msi::Value::Int(800)])
        .row(vec![msi::Value::Str("CostFinalize".into()), msi::Value::Null, msi::Value::Int(1000)])
    ).unwrap();
    package.flush().unwrap();
    package.into_inner().unwrap().into_inner()
}

fn test_msi(label: &str, data: Vec<u8>) {
    let path = format!("{}.msi", label);
    std::fs::write(&path, &data).unwrap();
    println!("--- {} ({} bytes) ---", label, data.len());

    // Quick verify with msi crate
    let file = std::fs::File::open(&path).unwrap();
    match msi::Package::open(file) {
        Ok(pkg) => {
            let tables: Vec<&str> = pkg.tables().map(|t| t.name()).collect();
            println!("  msi crate: OK, tables: {:?}", tables);
        }
        Err(e) => println!("  msi crate: FAIL: {}", e),
    }

    // Test with msiexec
    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart"])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    println!("  msiexec: exit code {}", code);

    // Cleanup
    if code == 0 {
        let _ = Command::new("msiexec").args(&["/x", &path, "/qn", "/norestart"]).output();
    }
    println!();
}
