/// Test: which ExecSeq actions trigger the 2705 error?
use velocity_msi::{MsiBuilder, Column, Value};
use std::process::Command;

fn build_msi(actions: &[(&str, i32)]) -> Vec<u8> {
    let mut b = MsiBuilder::new();
    b.set_title("ActionTest");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("ActionTest")],
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
    let rows: Vec<Vec<Value>> = actions.iter().map(|(name, seq)| {
        vec![Value::from(*name), Value::Null, Value::Int(*seq)]
    }).collect();
    b.insert_rows("InstallExecuteSequence", rows).unwrap();

    b.build().unwrap()
}

fn test_msi(data: &[u8], name: &str) -> (i32, String) {
    let path = format!("action_test_{}.msi", name);
    let log = format!("action_test_{}.log", name);
    std::fs::write(&path, data).unwrap();
    let _ = std::fs::remove_file(&log);
    let output = Command::new("msiexec")
        .args(&["/i", &path, "/qn", "/norestart", "/lv", &log])
        .output()
        .expect("msiexec failed");
    let code = output.status.code().unwrap_or(-1);
    let error = if code != 0 {
        std::fs::read_to_string(&log).ok()
            .and_then(|content| {
                content.lines().find(|l| l.contains("2705") || l.contains("DEBUG: Error"))
                    .map(|l| l.trim().to_string())
            })
            .unwrap_or_default()
    } else { String::new() };
    (code, error)
}

fn main() {
    println!("=== Testing which ExecSeq actions trigger 2705 ===\n");

    // Test 1: Only InstallValidate (non-standard action)
    let msi = build_msi(&[("InstallValidate", 1400)]);
    let (code, err) = test_msi(&msi, "validate");
    println!("InstallValidate(1400): exit {} {}", code, err);

    // Test 2: Only CostInitialize
    let msi = build_msi(&[("CostInitialize", 800)]);
    let (code, err) = test_msi(&msi, "costinit");
    println!("CostInitialize(800): exit {} {}", code, err);

    // Test 3: Only CostFinalize
    let msi = build_msi(&[("CostFinalize", 1000)]);
    let (code, err) = test_msi(&msi, "costfin");
    println!("CostFinalize(1000): exit {} {}", code, err);

    // Test 4: InstallValidate + CostInitialize + CostFinalize
    let msi = build_msi(&[
        ("InstallValidate", 1400),
        ("CostInitialize", 800),
        ("CostFinalize", 1000),
    ]);
    let (code, err) = test_msi(&msi, "all3");
    println!("All 3 actions: exit {} {}", code, err);

    // Test 5: DummyAction (non-standard)
    let msi = build_msi(&[("DummyAction", 100)]);
    let (code, err) = test_msi(&msi, "dummy");
    println!("DummyAction(100): exit {} {}", code, err);

    // Test 6: InstallInitialize
    let msi = build_msi(&[("InstallInitialize", 1500)]);
    let (code, err) = test_msi(&msi, "instinit");
    println!("InstallInitialize(1500): exit {} {}", code, err);

    // Test 7: InstallFinalize
    let msi = build_msi(&[("InstallFinalize", 6600)]);
    let (code, err) = test_msi(&msi, "instfin");
    println!("InstallFinalize(6600): exit {} {}", code, err);
}
