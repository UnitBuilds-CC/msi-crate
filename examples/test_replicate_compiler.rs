//! Replicate the compiler's exact MSI generation to find the bug.
//! Uses the `cab` crate for cabinet building (like the compiler does).

use std::process::Command;
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};

fn main() {
    println!("=== Replicate compiler MSI generation ===\n");

    let mut b = MsiBuilder::new();
    b.set_title("Sample App Installer");
    b.set_author("Velocity Team");
    b.set_subject("Sample App v1.0.0");
    b.set_comments("Sample App installer package");
    b.set_template("x64", 1033);

    // Create ALL tables exactly as the compiler does
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").string(38).nullable().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("KeyPath").string(72).nullable().build(),
    ]).unwrap();
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
    b.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int32().build(),
        Column::build("DiskPrompt").string(255).nullable().build(),
        Column::build("VolumeLabel").string(32).nullable().build(),
        Column::build("Cabinet").string(255).nullable().build(),
        Column::build("Source").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").string(38).nullable().build(),
        Column::build("Title").string(64).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Display").int16().nullable().build(),
        Column::build("Level").int16().nullable().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Attributes").int16().nullable().build(),
    ]).unwrap();
    b.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    b.create_table("Registry", vec![
        Column::build("Registry").string(72).primary_key().build(),
        Column::build("Root").int16().nullable().build(),
        Column::build("Key").string(255).nullable().build(),
        Column::build("Name").string(255).nullable().build(),
        Column::build("Value").string(255).nullable().build(),
        Column::build("Component_").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("Shortcut", vec![
        Column::build("Shortcut").string(72).primary_key().build(),
        Column::build("Directory_").string(72).nullable().build(),
        Column::build("Name").string(128).nullable().build(),
        Column::build("Component_").string(72).nullable().build(),
        Column::build("Target").string(255).nullable().build(),
        Column::build("Arguments").string(255).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
        Column::build("Hotkey").int16().nullable().build(),
        Column::build("Icon_").string(72).nullable().build(),
        Column::build("IconIndex").int16().nullable().build(),
        Column::build("ShowCmd").int16().nullable().build(),
        Column::build("WkDir").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("Icon", vec![
        Column::build("Name").string(72).primary_key().build(),
        Column::build("Data").binary().nullable().build(),
    ]).unwrap();
    b.create_table("Environment", vec![
        Column::build("Environment").string(72).primary_key().build(),
        Column::build("Name").string(255).nullable().build(),
        Column::build("Value").string(255).nullable().build(),
        Column::build("Component_").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("ServiceInstall", vec![
        Column::build("ServiceInstall").string(72).primary_key().build(),
        Column::build("Name").string(255).nullable().build(),
        Column::build("DisplayName").string(255).nullable().build(),
        Column::build("ServiceType").int32().nullable().build(),
        Column::build("StartType").int32().nullable().build(),
        Column::build("ErrorControl").int32().nullable().build(),
        Column::build("LoadOrderGroup").string(255).nullable().build(),
        Column::build("Dependencies").string(255).nullable().build(),
        Column::build("StartName").string(255).nullable().build(),
        Column::build("Password").string(255).nullable().build(),
        Column::build("Arguments").string(255).nullable().build(),
        Column::build("Component_").string(72).nullable().build(),
        Column::build("Description").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("ServiceControl", vec![
        Column::build("ServiceControl").string(72).primary_key().build(),
        Column::build("Name").string(255).nullable().build(),
        Column::build("Event").int32().nullable().build(),
        Column::build("Arguments").string(255).nullable().build(),
        Column::build("Wait").int16().nullable().build(),
        Column::build("Component_").string(72).nullable().build(),
    ]).unwrap();
    b.create_table("CustomAction", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Type").int16().nullable().build(),
        Column::build("Source").string(72).nullable().build(),
        Column::build("Target").string(255).nullable().build(),
    ]).unwrap();
    b.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").string(255).nullable().build(),
        Column::build("Sequence").int16().nullable().build(),
    ]).unwrap();
    b.create_table("Upgrade", vec![
        Column::build("UpgradeCode").string(38).primary_key().build(),
        Column::build("VersionMin").string(20).nullable().build(),
        Column::build("VersionMax").string(20).nullable().build(),
        Column::build("Language").string(20).nullable().build(),
        Column::build("Attributes").int32().nullable().build(),
    ]).unwrap();
    b.create_table("LaunchCondition", vec![
        Column::build("Condition").string(255).primary_key().build(),
        Column::build("Description").string(255).nullable().build(),
    ]).unwrap();

    // Populate Property table (exactly as compiler does)
    let props = vec![
        ("ProductCode", "{D5E0EEC4-CF5C-5D68-BAC7-AA26667F6C80}"),
        ("UpgradeCode", "{7B44FAB1-58DD-5368-9B0C-338B5E7519DD}"),
        ("ProductName", "Sample App"),
        ("Manufacturer", "Velocity Team"),
        ("ProductVersion", "1.0.0"),
        ("ProductLanguage", "1033"),
        ("ALLUSERS", "1"),
        ("Description", "Sample App Installer"),
    ];
    for (name, value) in &props {
        b.insert_rows("Property", vec![
            vec![Value::from(*name), Value::from(*value)],
        ]).unwrap();
    }

    // Populate Directory table
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFiles64Folder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFiles64Folder"), Value::from("SampleApp:SampleApp")],
        vec![Value::from("ProgramMenuFolder"), Value::from("TARGETDIR"), Value::from("Programs")],
        vec![Value::from("ApplicationProgramsFolder"), Value::from("ProgramMenuFolder"), Value::from("SampleApp:SampleApp")],
        vec![Value::from("DesktopFolder"), Value::from("TARGETDIR"), Value::from("Desktop")],
    ]).unwrap();

    // Populate 9 files (like the compiler)
    let file_names = vec![
        "core.dll", "sample-app.exe", "version.txt",
        "api-reference.pdf", "manual.pdf",
        "sample.h", "sample.lib",
        "example1.txt", "example2.txt",
    ];
    
    let mut file_rows = Vec::new();
    let mut comp_rows = Vec::new();
    let mut fc_rows = Vec::new();
    
    for (i, fname) in file_names.iter().enumerate() {
        let file_id = format!("file_{}", i);
        let comp_id = format!("comp_{}", i);
        let guid = format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
            0xA1B20000 + i as u32, 0xC3D4, 0x5678, 0x9ABC, 0xDEF012340000u64 + i as u64);
        
        comp_rows.push(vec![
            Value::from(comp_id.as_str()),
            Value::from(guid.as_str()),
            Value::from("INSTALLDIR"),
            Value::Int(0i32),
            Value::Null,
            Value::from(file_id.as_str()),
        ]);
        file_rows.push(vec![
            Value::from(file_id.as_str()),
            Value::from(comp_id.as_str()),
            Value::from(*fname),
            Value::from(100i32),  // fake size
            Value::Null, Value::Null,
            Value::Int(0i32),
            Value::from((i + 1) as i32),
        ]);
        fc_rows.push(vec![
            Value::from("Complete"),
            Value::from(comp_id.as_str()),
        ]);
    }
    
    b.insert_rows("Component", comp_rows).unwrap();
    b.insert_rows("File", file_rows).unwrap();
    b.insert_rows("Feature", vec![
        vec![Value::from("Complete"), Value::Null, Value::from("Complete"),
             Value::from("All features"), Value::from(1i32), Value::from(1i32),
             Value::Null, Value::from(0i32)],
    ]).unwrap();
    b.insert_rows("FeatureComponents", fc_rows).unwrap();

    // Media
    b.insert_rows("Media", vec![
        vec![Value::from(1i32), Value::from(9i32), Value::Null, Value::Null,
             Value::from("#Velocity.cab"), Value::Null],
    ]).unwrap();

    // Registry
    b.insert_rows("Registry", vec![
        vec![Value::from("reg_0"), Value::from(2i32), Value::from("Software\\SampleApp"),
             Value::from("InstallPath"), Value::from("[INSTALLDIR]"), Value::from("comp_0")],
        vec![Value::from("reg_1"), Value::from(2i32), Value::from("Software\\SampleApp"),
             Value::from("Version"), Value::from("1.0.0"), Value::from("comp_0")],
        vec![Value::from("reg_2"), Value::from(1i32), Value::from("Software\\SampleApp\\Settings"),
             Value::from("FirstRun"), Value::from("1"), Value::from("comp_0")],
    ]).unwrap();

    // Shortcuts
    b.insert_rows("Shortcut", vec![
        vec![Value::from("DesktopShortcut"), Value::from("DesktopFolder"),
             Value::from("Sample App"), Value::from("comp_desktop_shortcut"),
             Value::from("[INSTALLDIR]"), Value::Null, Value::from("Sample App"),
             Value::Int(0i32), Value::Null, Value::Int(0i32), Value::Int(1i32),
             Value::from("INSTALLDIR")],
        vec![Value::from("StartMenuShortcut"), Value::from("ApplicationProgramsFolder"),
             Value::from("Sample App"), Value::from("comp_startmenu_shortcut"),
             Value::from("[INSTALLDIR]"), Value::Null, Value::from("Sample App"),
             Value::Int(0i32), Value::Null, Value::Int(0i32), Value::Int(1i32),
             Value::from("INSTALLDIR")],
    ]).unwrap();

    // Components for shortcuts (KeyPath must be Null for non-file components)
    b.insert_rows("Component", vec![
        vec![Value::from("comp_desktop_shortcut"), Value::from("{B1C2D3E4-F5A6-7890-ABCD-EF1234567890}"),
             Value::from("DesktopFolder"), Value::Int(0i32), Value::Null, Value::Null],
        vec![Value::from("comp_startmenu_shortcut"), Value::from("{C1D2E3F4-A5B6-7890-ABCD-EF1234567890}"),
             Value::from("ApplicationProgramsFolder"), Value::Int(0i32), Value::Null, Value::Null],
    ]).unwrap();
    b.insert_rows("FeatureComponents", vec![
        vec![Value::from("Complete"), Value::from("comp_desktop_shortcut")],
        vec![Value::from("Complete"), Value::from("comp_startmenu_shortcut")],
    ]).unwrap();

    // Environment
    b.insert_rows("Environment", vec![
        vec![Value::from("env_0"), Value::from("SAMPLE_APP_HOME"), Value::from("[INSTALLDIR]"), Value::from("comp_0")],
        vec![Value::from("env_1"), Value::from("SAMPLE_APP_VERSION"), Value::from("1.0.0"), Value::from("comp_0")],
    ]).unwrap();

    // CustomAction
    b.insert_rows("CustomAction", vec![
        vec![Value::from("ca_pre_install"), Value::from(51i32), Value::from("TARGETDIR"), Value::from("[INSTALLDIR]")],
        vec![Value::from("ca_post_install"), Value::from(51i32), Value::from("TARGETDIR"), Value::from("[INSTALLDIR]")],
    ]).unwrap();

    // InstallExecuteSequence
    b.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
        vec![Value::from("InstallValidate"), Value::Null, Value::from(1400i32)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::from(1500i32)],
        vec![Value::from("ProcessComponents"), Value::Null, Value::from(1600i32)],
        vec![Value::from("InstallFiles"), Value::Null, Value::from(4000i32)],
        vec![Value::from("RegisterProduct"), Value::Null, Value::from(5700i32)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::from(6600i32)],
        vec![Value::from("ca_pre_install"), Value::Null, Value::from(450i32)],
        vec![Value::from("ca_post_install"), Value::Null, Value::from(6700i32)],
    ]).unwrap();

    // InstallUISequence
    b.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::from(800i32)],
        vec![Value::from("CostFinalize"), Value::Null, Value::from(1000i32)],
    ]).unwrap();

    // Build cabinet using custom builder (proven to work)
    let cab_files: Vec<CabinetFile> = (0..9).map(|i| {
        CabinetFile {
            name: format!("file_{}", i),
            data: format!("Fake content for file {}", i).into_bytes(),
        }
    }).collect();
    let cab_bytes = build_cabinet(&cab_files);
    println!("Cabinet size: {} bytes", cab_bytes.len());
    
    b.add_stream("Velocity.cab".to_string(), cab_bytes);

    // Build MSI
    let msi_data = b.build().unwrap();
    let msi_path = "C:\\temp\\vel_msi_test\\replicated.msi";
    std::fs::create_dir_all("C:\\temp\\vel_msi_test").ok();
    std::fs::write(msi_path, &msi_data).unwrap();
    println!("MSI written: {} bytes", msi_data.len());

    // Test
    let install_dir = "C:\\temp\\vel_msi_install\\replicated";
    let log_path = "C:\\temp\\vel_msi_test\\replicated.log";
    let status = Command::new("msiexec")
        .args(&["/i", msi_path, "/qn", "/l*v", log_path,
                &format!("TARGETDIR={}", install_dir)])
        .status().unwrap();
    let code = status.code().unwrap_or(-1);
    println!("Exit code: {}", code);

    if code != 0 {
        if let Ok(log) = std::fs::read_to_string(log_path) {
            let lines: Vec<&str> = log.lines().collect();
            let start = lines.len().saturating_sub(25);
            for line in &lines[start..] {
                println!("  {}", line);
            }
        }
    } else {
        println!("SUCCESS!");
        let _ = Command::new("msiexec").args(&["/x", msi_path, "/qn"]).status();
    }
    let _ = std::fs::remove_dir_all(install_dir);
}
