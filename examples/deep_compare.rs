/// Deep comparison: read our MSI with msi crate and dump ALL metadata
/// to find what msiexec might be rejecting.
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn main() {
    // Build the failing MSI
    let mut b = MsiBuilder::new();
    b.set_title("DeepDiag");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("DeepDiag")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("UpgradeCode"), Value::from("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
    ]).unwrap();

    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
    ]).unwrap();

    let msi_data = b.build().unwrap();
    std::fs::write("deep_diag.msi", &msi_data).unwrap();
    println!("MSI size: {} bytes\n", msi_data.len());

    // Read with msi crate
    let cursor = std::io::Cursor::new(msi_data.clone());
    let mut package = msi::Package::open(cursor).expect("open failed");

    // Dump _Tables
    println!("=== _Tables ===");
    for row in package.select_rows(msi::Select::table("_Tables")).expect("read _Tables") {
        println!("  Name={:?}", row[0].as_str());
    }

    // Dump _Columns for Directory table specifically
    println!("\n=== _Columns for Directory ===");
    for row in package.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Directory" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            println!("  Table={}, Number={}, Name={}, Type=0x{:04X}", table, number, name, type_val);
            // Decode the type bitfield
            let base = type_val & 0xFFF;
            let nullable = (type_val & 0x1000) != 0;
            let pk = (type_val & 0x2000) != 0;
            println!("    base=0x{:03X}, nullable={}, pk={}", base, nullable, pk);
        }
    }

    // Dump _Columns for ALL tables
    println!("\n=== _Columns (all) ===");
    for row in package.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        let number = row[1].as_int().unwrap_or(-1);
        let name = row[2].as_str().unwrap_or("?");
        let type_val = row[3].as_int().unwrap_or(-1);
        println!("  T={} N={} Col={} Type=0x{:04X}", table, number, name, type_val);
    }

    // Dump _Validation for Directory
    println!("\n=== _Validation for Directory ===");
    for row in package.select_rows(msi::Select::table("_Validation")).expect("read _Validation") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Directory" {
            let col = row[1].as_str().unwrap_or("?");
            let nullable = row[2].as_str().unwrap_or("?");
            let min = row[3].as_int();
            let max = row[4].as_int();
            let key_table = row[5].as_str();
            let key_col = row[6].as_int();
            let category = row[7].as_str();
            let set = row[8].as_str();
            let desc = row[9].as_str();
            println!("  Col={}, Nullable={}, Min={:?}, Max={:?}, KeyTable={:?}, KeyCol={:?}, Cat={:?}, Set={:?}, Desc={:?}",
                col, nullable, min, max, key_table, key_col, category, set, desc);
        }
    }

    // Dump Directory table
    println!("\n=== Directory ===");
    for row in package.select_rows(msi::Select::table("Directory")).expect("read Directory") {
        let dir = row[0].as_str().unwrap_or("?");
        let parent = row[1].as_str();
        let default = row[2].as_str().unwrap_or("?");
        println!("  Dir={}, Parent={:?}, Default={}", dir, parent, default);
    }

    // Dump InstallExecuteSequence
    println!("\n=== InstallExecuteSequence ===");
    for row in package.select_rows(msi::Select::table("InstallExecuteSequence")).expect("read ExecSeq") {
        let action = row[0].as_str().unwrap_or("?");
        let cond = row[1].as_str();
        let seq = row[2].as_int();
        println!("  Action={}, Condition={:?}, Sequence={:?}", action, cond, seq);
    }

    // Dump Property
    println!("\n=== Property ===");
    for row in package.select_rows(msi::Select::table("Property")).expect("read Property") {
        let prop = row[0].as_str().unwrap_or("?");
        let val = row[1].as_str().unwrap_or("?");
        println!("  {}={}", prop, val);
    }

    // Now let's also create a reference MSI with the msi crate and compare
    println!("\n=== Creating reference MSI with msi crate ===");
    let ref_cursor = std::io::Cursor::new(Vec::new());
    let mut ref_pkg = msi::Package::create(msi::PackageType::Installer, ref_cursor).expect("create");
    
    ref_pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().string(72),
        msi::Column::build("Value").nullable().string(255),
    ]).expect("create Property");
    ref_pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("DeepDiag".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("V".into())])
        .row(vec![msi::Value::Str("ProductCode".into()), msi::Value::Str("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}".into())])
        .row(vec![msi::Value::Str("UpgradeCode".into()), msi::Value::Str("{BBDDE5F5-4B23-5C8E-B5F1-6B3A9C4D7EA1}".into())])
        .row(vec![msi::Value::Str("ProductLanguage".into()), msi::Value::Str("1033".into())])
    ).expect("insert Property");

    ref_pkg.create_table("Directory", vec![
        msi::Column::build("Directory").primary_key().string(72),
        msi::Column::build("Directory_Parent").nullable().string(72),
        msi::Column::build("DefaultDir").primary_key().string(255),
    ]).expect("create Directory");
    ref_pkg.insert_rows(msi::Insert::into("Directory")
        .row(vec![
            msi::Value::Str("TARGETDIR".into()),
            msi::Value::Null,
            msi::Value::Str("SourceDir".into()),
        ])
    ).expect("insert Directory");

    ref_pkg.create_table("InstallExecuteSequence", vec![
        msi::Column::build("Action").primary_key().string(72),
        msi::Column::build("Condition").nullable().string(255),
        msi::Column::build("Sequence").nullable().int16(),
    ]).expect("create ExecSeq");
    ref_pkg.insert_rows(msi::Insert::into("InstallExecuteSequence")
        .row(vec![
            msi::Value::Str("CostInitialize".into()),
            msi::Value::Null,
            msi::Value::Int(800),
        ])
    ).expect("insert ExecSeq");

    let ref_data = ref_pkg.into_inner().expect("into_inner").into_inner();
    println!("Reference MSI size: {} bytes", ref_data.len());

    // Read reference MSI _Columns for Directory
    let ref_cursor2 = std::io::Cursor::new(ref_data.clone());
    let mut ref_pkg2 = msi::Package::open(ref_cursor2).expect("open ref");
    println!("\n=== Reference _Columns for Directory ===");
    for row in ref_pkg2.select_rows(msi::Select::table("_Columns")).expect("read _Columns") {
        let table = row[0].as_str().unwrap_or("?");
        if table == "Directory" {
            let number = row[1].as_int().unwrap_or(-1);
            let name = row[2].as_str().unwrap_or("?");
            let type_val = row[3].as_int().unwrap_or(-1);
            println!("  Table={}, Number={}, Name={}, Type=0x{:04X}", table, number, name, type_val);
        }
    }

    // Compare _Tables
    println!("\n=== Reference _Tables ===");
    for row in ref_pkg2.select_rows(msi::Select::table("_Tables")).expect("read _Tables") {
        println!("  Name={:?}", row[0].as_str());
    }

    // Test reference MSI with msiexec
    std::fs::write("deep_diag_ref.msi", &ref_data).unwrap();
    let _ = std::fs::remove_file("deep_diag_ref.log");
    let output = Command::new("msiexec")
        .args(&["/i", "deep_diag_ref.msi", "/qn", "/norestart", "/lv", "deep_diag_ref.log"])
        .output().expect("msiexec");
    println!("\nReference MSI msiexec exit: {}", output.status.code().unwrap_or(-1));
    if let Ok(log) = std::fs::read_to_string("deep_diag_ref.log") {
        for line in log.lines() {
            if line.contains("1620") || line.contains("2705") || line.contains("DEBUG: Error") {
                println!("  {}", line.trim());
            }
        }
    }
}
