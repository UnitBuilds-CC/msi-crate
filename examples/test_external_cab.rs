//! Test: external cabinet file to isolate cabinet format vs embedding issue.
//! If external cabinet works, the issue is with how we embed it in the MSI.
//! If external cabinet also fails, the issue is with the cabinet format itself.

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    println!("=== Test: External cabinet file ===\n");

    let test_dir = "C:\\temp\\cab_test";
    std::fs::create_dir_all(test_dir).ok();

    let test_content = b"Hello from Velocity MSI!";
    let test_file_name = "velocity_test.txt";

    // Build cabinet
    let cabinet = build_cabinet(&[
        CabinetFile {
            name: "F1".to_string(),  // Must match File.File primary key
            data: test_content.to_vec(),
        },
    ]);

    // Write cabinet to disk
    let cab_path = format!("{}\\velo.cab", test_dir);
    std::fs::write(&cab_path, &cabinet).unwrap();
    println!("Cabinet written to: {} ({} bytes)", cab_path, cabinet.len());

    // Dump hex of first 64 bytes
    println!("\nCabinet hex dump (first 64 bytes):");
    for (i, chunk) in cabinet.chunks(16).enumerate() {
        if i >= 4 { break; }
        print!("  {:04x}: ", i * 16);
        for b in chunk {
            print!("{:02x} ", b);
        }
        print!(" ");
        for b in chunk {
            if *b >= 0x20 && *b < 0x7f {
                print!("{}", *b as char);
            } else {
                print!(".");
            }
        }
        println!();
    }

    // Try to verify with Windows expand.exe
    println!("\n--- Verifying cabinet with expand.exe ---");
    let expand_dir = format!("{}\\expand_out", test_dir);
    std::fs::create_dir_all(&expand_dir).ok();
    let status = Command::new("expand")
        .args(&[&cab_path, &expand_dir, "-F:*"])
        .output();
    match status {
        Ok(output) => {
            println!("expand exit code: {}", output.status.code().unwrap_or(-1));
            println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
            println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
        }
        Err(e) => println!("Failed to run expand: {}", e),
    }

    // Test 1: External cabinet (Media.Source points to directory)
    println!("\n--- Test 1: External cabinet ---");
    {
        let mut b = MsiBuilder::new();
        b.set_title("Cab Test External");
        b.set_author("Velocity");
        b.set_template("x64", 1033);

        // Property table
        b.create_table("Property", vec![
            Column::build("Property").string(72).primary_key().build(),
            Column::build("Value").string(255).nullable().build(),
        ]).unwrap();
        b.insert_rows("Property", vec![
            vec![Value::from("ProductName"), Value::from("Cab Test")],
            vec![Value::from("ProductCode"), Value::from("{A1B2C3D4-1111-2222-3333-444455556666}")],
            vec![Value::from("ProductVersion"), Value::from("1.0.0")],
            vec![Value::from("ProductLanguage"), Value::from("1033")],
            vec![Value::from("Manufacturer"), Value::from("Velocity")],
            vec![Value::from("UpgradeCode"), Value::from("{A1B2C3D4-1111-2222-3333-444455557777}")],
        ]).unwrap();

        // Directory table
        b.create_table("Directory", vec![
            Column::build("Directory").string(72).primary_key().build(),
            Column::build("Directory_Parent").string(72).nullable().build(),
            Column::build("DefaultDir").string(255).build(),
        ]).unwrap();
        b.insert_rows("Directory", vec![
            vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
            vec![Value::from("VelocityDir"), Value::from("TARGETDIR"), Value::from("VelocityTest")],
        ]).unwrap();

        // Component table
        b.create_table("Component", vec![
            Column::build("Component").string(72).primary_key().build(),
            Column::build("ComponentId").string(72).nullable().build(),
            Column::build("Directory_").string(72).build(),
            Column::build("Attributes").int16().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("KeyPath").string(72).nullable().build(),
        ]).unwrap();
        b.insert_rows("Component", vec![
            vec![
                Value::from("MainComp"),
                Value::from("{A1B2C3D4-1111-2222-3333-444455558888}"),
                Value::from("VelocityDir"),
                Value::from(0i32),
                Value::Null,
                Value::from("F1"),
            ],
        ]).unwrap();

        // Feature table
        b.create_table("Feature", vec![
            Column::build("Feature").string(72).primary_key().build(),
            Column::build("Feature_Parent").string(72).nullable().build(),
            Column::build("Title").string(255).nullable().build(),
            Column::build("Description").string(255).nullable().build(),
            Column::build("Display").int16().nullable().build(),
            Column::build("Level").int16().build(),
            Column::build("Directory_").string(72).nullable().build(),
            Column::build("Attributes").int16().build(),
        ]).unwrap();
        b.insert_rows("Feature", vec![
            vec![
                Value::from("MainFeature"), Value::Null, Value::from("Complete"),
                Value::from("All files"), Value::from(1i32), Value::from(1i32),
                Value::Null, Value::from(0i32),
            ],
        ]).unwrap();

        // FeatureComponents table
        b.create_table("FeatureComponents", vec![
            Column::build("Feature_").string(72).primary_key().build(),
            Column::build("Component_").string(72).primary_key().build(),
        ]).unwrap();
        b.insert_rows("FeatureComponents", vec![
            vec![Value::from("MainFeature"), Value::from("MainComp")],
        ]).unwrap();

        // File table (8 columns)
        b.create_table("File", vec![
            Column::build("File").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).build(),
            Column::build("FileSize").int32().build(),
            Column::build("Version").string(72).nullable().build(),
            Column::build("Language").int16().nullable().build(),
            Column::build("Attributes").int16().nullable().build(),
            Column::build("Sequence").int32().build(),
        ]).unwrap();
        b.insert_rows("File", vec![
            vec![
                Value::from("F1"),
                Value::from("MainComp"),
                Value::from(test_file_name),
                Value::from(test_content.len() as i32),
                Value::Null, Value::Null, Value::from(0i32), Value::from(1i32),
            ],
        ]).unwrap();

        // Media table - external cabinet (no # prefix, Source points to directory)
        b.create_table("Media", vec![
            Column::build("DiskId").int16().primary_key().build(),
            Column::build("LastSequence").int32().build(),
            Column::build("DiskPrompt").string(255).nullable().build(),
            Column::build("VolumeLabel").string(32).nullable().build(),
            Column::build("Cabinet").string(255).nullable().build(),
            Column::build("Source").string(72).nullable().build(),
        ]).unwrap();
        b.insert_rows("Media", vec![
            vec![
                Value::from(1i32),
                Value::from(1i32),
                Value::Null,
                Value::Null,
                Value::from("velo.cab"),    // Cabinet name (no # = external)
                Value::Null,                // Source (same dir as MSI)
            ],
        ]).unwrap();

        // InstallExecuteSequence
        b.create_table("InstallExecuteSequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallExecuteSequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
            vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
            vec![Value::from("InstallValidate"), Value::Null, Value::from(1400i32)],
            vec![Value::from("InstallInitialize"), Value::Null, Value::from(1500i32)],
            vec![Value::from("ProcessComponents"), Value::Null, Value::from(1600i32)],
            vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
            vec![Value::from("RegisterProduct"), Value::Null, Value::from(5700i32)],
            vec![Value::from("InstallFinalize"), Value::Null, Value::from(6600i32)],
        ]).unwrap();

        // InstallUISequence
        b.create_table("InstallUISequence", vec![
            Column::build("Action").string(72).primary_key().build(),
            Column::build("Condition").string(255).nullable().build(),
            Column::build("Sequence").int16().nullable().build(),
        ]).unwrap();
        b.insert_rows("InstallUISequence", vec![
            vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
            vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        ]).unwrap();

        // Copy cabinet to same directory as MSI
        let msi_path = format!("{}\\external_test.msi", test_dir);
        let msi_data = b.build().unwrap();
        std::fs::write(&msi_path, &msi_data).unwrap();
        println!("MSI written to: {} ({} bytes)", msi_path, msi_data.len());

        let target_dir = "C:\\temp\\cab_install_test";
        std::fs::create_dir_all(target_dir).ok();

        let log_path = format!("{}\\external.log", test_dir);
        let status = Command::new("msiexec")
            .args(&[
                "/i", &msi_path, "/qn", "/norestart",
                "/l*v", &log_path,
                &format!("TARGETDIR={}", target_dir),
            ])
            .status();
        let code = match status {
            Ok(s) => s.code().unwrap_or(-1),
            Err(_) => -1,
        };
        println!("External cabinet: exit code {}", code);
        if code != 0 {
            // Show relevant errors
            if let Ok(log) = std::fs::read_to_string(&log_path) {
                for line in log.lines() {
                    if line.contains("Error ") && (line.contains("1334") || line.contains("cabinet") || line.contains("1303")) {
                        println!("  LOG: {}", line.trim());
                    }
                }
            }
        }
    }

    println!("\nDone.");
}
