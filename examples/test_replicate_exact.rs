//! Replicate compiler's exact MSI build flow with sample-app data
use velocity_msi::{MsiBuilder, Column, Value, CabinetFile, build_cabinet};
use std::process::Command;

fn main() {
    println!("=== Replicate compiler MSI exactly ===\n");

    let mut b = MsiBuilder::new();
    b.set_title("Sample App Installer");
    b.set_author("Velocity Team");
    b.set_subject("Sample App v1.0.0");
    b.set_comments("Sample App installer package");
    b.set_template("x64", 1033);

    // Create ALL tables (same as compiler)
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
    ]).unwrap();
    b.create_table("ServiceControl", vec![
        Column::build("ServiceControl").string(72).primary_key().build(),
        Column::build("Name").string(255).nullable().build(),
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

    // Populate data exactly like compiler
    let product_code = "{D5E0EEC4-CF5C-5D68-BAC7-AA26667F6C80}";
    let upgrade_code = "{7B44FAB1-58DD-5368-9B0C-338B5E7519DD}";

    // Properties
    let props: Vec<(&str, &str)> = vec![
        ("ProductCode", product_code),
        ("UpgradeCode", upgrade_code),
        ("ProductName", "Sample App"),
        ("Manufacturer", "Velocity Team"),
        ("ProductVersion", "1.0.0"),
        ("ProductLanguage", "1033"),
        ("ALLUSERS", "1"),
        ("ARPPRODUCTICON", "AppIcon.ico"),
    ];
    for (name, value) in props {
        b.insert_rows("Property", vec![vec![Value::from(name), Value::from(value)]]).unwrap();
    }
    b.insert_rows("Property", vec![vec![Value::from("Description"), Value::from("A sample application")]]).unwrap();

    // Directories
    b.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFiles64Folder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFiles64Folder"), Value::from("SampleApp:SAMPLEAPP")],
        vec![Value::from("ProgramMenuFolder"), Value::from("TARGETDIR"), Value::from("Programs")],
        vec![Value::from("ApplicationProgramsFolder"), Value::from("ProgramMenuFolder"), Value::from("SampleApp:SAMPLEAPP")],
        vec![Value::from("DesktopFolder"), Value::from("TARGETDIR"), Value::from("Desktop")],
    ]).unwrap();

    // 9 file components (with short|long filename format like compiler)
    let file_names = vec![
        "SAMPLE~1.EXE|sample-app.exe", "CORE~1.DLL|core.dll", "VERSIO~1.TXT|version.txt",
        "APIREF~1.PDF|api-reference.pdf", "MANUAL~1.PDF|manual.pdf",
        "EXAMPL~1.TXT|example1.txt", "EXAMPL~2.TXT|example2.txt",
        "SAMPLE~1.H|sample.h", "SAMPLE~1.LIB|sample.lib",
    ];
    for (i, fname) in file_names.iter().enumerate() {
        let comp_id = format!("comp_{}", i);
        let file_id = format!("file_{}", i);
        let guid = format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
            0xAAAAAAAA + i as u32, 0xBBBB, 0xCCCC, 0xDDDD, 0xEEEEEEEEEE + i as u64);
        b.insert_rows("Component", vec![vec![
            Value::from(comp_id.as_str()), Value::from(guid.as_str()),
            Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::from(file_id.as_str()),
        ]]).unwrap();
        b.insert_rows("File", vec![vec![
            Value::from(file_id.as_str()), Value::from(comp_id.as_str()),
            Value::from(*fname), Value::Int(1024), Value::Null, Value::Int(0), Value::Int(0), Value::Int((i+1) as i32),
        ]]).unwrap();
        b.insert_rows("FeatureComponents", vec![vec![
            Value::from("Complete"), Value::from(comp_id.as_str()),
        ]]).unwrap();
    }

    // Feature
    b.insert_rows("Feature", vec![vec![
        Value::from("Complete"), Value::Null, Value::from("Sample App Setup"),
        Value::from("Complete installation of Sample App"),
        Value::Int(1), Value::Int(1), Value::from("INSTALLDIR"), Value::Int(0),
    ]]).unwrap();

    // Media
    b.insert_rows("Media", vec![vec![
        Value::Int(1), Value::Int(9), Value::Null, Value::Null,
        Value::from("#Velocity.cab"), Value::Null,
    ]]).unwrap();

    // Registry (3 entries with components)
    for i in 0..3 {
        let reg_id = format!("reg_{}", i);
        let comp_id = format!("comp_reg_{}", i);
        let guid = format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
            0x11111111 + i as u32, 0x2222, 0x3333, 0x4444, 0x555555555555 + i as u64);
        b.insert_rows("Component", vec![vec![
            Value::from(comp_id.as_str()), Value::from(guid.as_str()),
            Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null,
        ]]).unwrap();
        b.insert_rows("FeatureComponents", vec![vec![
            Value::from("Complete"), Value::from(comp_id.as_str()),
        ]]).unwrap();
        b.insert_rows("Registry", vec![vec![
            Value::from(reg_id.as_str()), Value::Int(2),
            Value::from("Software\\SampleApp"),
            Value::from(format!("Value{}", i).as_str()),
            Value::from(format!("Data{}", i).as_str()),
            Value::from(comp_id.as_str()),
        ]]).unwrap();
    }

    // Shortcut (desktop + start menu) with components
    for (sid, dir) in &[("DesktopShortcut", "DesktopFolder"), ("StartMenuShortcut", "ApplicationProgramsFolder")] {
        let comp_id = format!("comp_shortcut_{}", sid);
        let guid = format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
            0x33333333, 0x4444, 0x5555, 0x6666, if *dir == "DesktopFolder" { 0x777777777777u64 } else { 0x888888888888u64 });
        b.insert_rows("Component", vec![vec![
            Value::from(comp_id.as_str()), Value::from(guid.as_str()),
            Value::from(*dir), Value::Int(0), Value::Null, Value::Null,
        ]]).unwrap();
        b.insert_rows("FeatureComponents", vec![vec![
            Value::from("Complete"), Value::from(comp_id.as_str()),
        ]]).unwrap();
        b.insert_rows("Shortcut", vec![vec![
            Value::from(*sid), Value::from(*dir), Value::from("Sample App"),
            Value::from(comp_id.as_str()), Value::from("[INSTALLDIR]sample-app.exe"),
            Value::Null, Value::from("A sample application"),
            Value::Int(0), Value::Null, Value::Int(0), Value::Int(1), Value::from("INSTALLDIR"),
        ]]).unwrap();
    }

    // Environment (2 entries with components)
    for i in 0..2 {
        let env_id = format!("env_{}", i);
        let comp_id = format!("comp_env_{}", i);
        let guid = format!("{:08X}-{:04X}-{:04X}-{:04X}-{:012X}",
            0x44444444 + i as u32, 0x5555, 0x6666, 0x7777, 0x888888888888 + i as u64);
        b.insert_rows("Component", vec![vec![
            Value::from(comp_id.as_str()), Value::from(guid.as_str()),
            Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null,
        ]]).unwrap();
        b.insert_rows("FeatureComponents", vec![vec![
            Value::from("Complete"), Value::from(comp_id.as_str()),
        ]]).unwrap();
        b.insert_rows("Environment", vec![vec![
            Value::from(env_id.as_str()), Value::from("SAMPLE_HOME"),
            Value::from("[INSTALLDIR]"), Value::from(comp_id.as_str()),
        ]]).unwrap();
    }

    // InstallExecuteSequence
    let exec_actions: Vec<(&str, Option<&str>, i32)> = vec![
        ("AppSearch", None, 100), ("LaunchConditions", Some("NOT Installed"), 105),
        ("ValidateProductID", None, 110), ("CostInitialize", None, 120),
        ("FileCost", None, 130), ("CostFinalize", None, 140),
        ("InstallValidate", None, 150), ("InstallInitialize", None, 160),
        ("ProcessComponents", None, 170), ("InstallFiles", None, 200),
        ("InstallShortcuts", None, 210), ("WriteRegistryValues", None, 220),
        ("WriteEnvironmentStrings", None, 230), ("InstallFinalize", None, 400),
        ("RegisterProduct", None, 300), ("PublishProduct", None, 310),
    ];
    for (action, cond, seq) in exec_actions {
        let c: Value = match cond { Some(v) => Value::from(v), None => Value::Null };
        b.insert_rows("InstallExecuteSequence", vec![vec![
            Value::from(action), c, Value::Int(seq),
        ]]).unwrap();
    }

    // InstallUISequence
    let ui_actions: Vec<(&str, Option<&str>, i32)> = vec![
        ("ShowLog", None, -1), ("ProgressDlg", None, 10), ("CancelDlg", None, 15),
        ("ErrorDlg", None, 50), ("FatalError", None, 9999), ("UserExit", None, 9999),
        ("WelcomeDlg", Some("NOT Installed"), 1230),
        ("LicenseAgreementDlg", Some("NOT Installed"), 1235),
        ("InstallDirDlg", Some("NOT Installed"), 1240),
        ("VerifyReadyDlg", Some("NOT Installed"), 1250),
        ("MaintenanceWelcomeDlg", Some("Installed"), 1260),
        ("MaintenanceTypeDlg", Some("Installed"), 1265),
        ("VerifyRepairDlg", Some("Installed"), 1270),
        ("ActionText", None, 20), ("ExecuteAction", None, 1300),
    ];
    for (action, cond, seq) in ui_actions {
        let c: Value = match cond { Some(v) => Value::from(v), None => Value::Null };
        b.insert_rows("InstallUISequence", vec![vec![
            Value::from(action), c, Value::Int(seq),
        ]]).unwrap();
    }

    // CustomActions (matching compiler's pre/post install + run_after)
    b.insert_rows("CustomAction", vec![vec![
        Value::from("PreInstallCmd_0"), Value::Int(34), Value::Null,
        Value::from("cmd.exe /c echo Installing Sample App..."),
    ]]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![vec![
        Value::from("PreInstallCmd_0"), Value::from("NOT Installed"), Value::Int(155),
    ]]).unwrap();
    b.insert_rows("CustomAction", vec![vec![
        Value::from("PostInstallCmd_0"), Value::Int(34), Value::Null,
        Value::from("cmd.exe /c echo Installed!"),
    ]]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![vec![
        Value::from("PostInstallCmd_0"), Value::from("NOT Installed"), Value::Int(401),
    ]]).unwrap();
    b.insert_rows("CustomAction", vec![vec![
        Value::from("LaunchApplication"), Value::Int(34), Value::Null,
        Value::from("[INSTALLDIR]sample-app.exe"),
    ]]).unwrap();
    b.insert_rows("InstallExecuteSequence", vec![vec![
        Value::from("LaunchApplication"), Value::from("NOT Installed"), Value::Int(450),
    ]]).unwrap();

    // Cabinet
    let mut cab_files = Vec::new();
    for (i, fname) in file_names.iter().enumerate() {
        cab_files.push(CabinetFile {
            name: format!("file_{}", i),
            data: vec![0u8; 1024],
        });
    }
    let cabinet = build_cabinet(&cab_files);
    b.add_stream("Velocity.cab".to_string(), cabinet);

    // Build
    let msi = b.build().unwrap();
    let outpath = "replicate_compiler.msi";
    std::fs::write(outpath, &msi).unwrap();
    println!("Built: {} ({} bytes)", outpath, msi.len());

    // Test with msiexec
    let log = "replicate_compiler.log";
    let status = Command::new("msiexec.exe")
        .args(&["/i", outpath, "/qn", "/l*v", log])
        .status()
        .unwrap();
    let code = status.code().unwrap_or(-1);
    println!("msiexec exit code: {}", code);

    // Check log
    if let Ok(log_content) = std::fs::read_to_string(log) {
        for line in log_content.lines() {
            if line.contains("installed the product") || line.contains("1620") || line.contains("Product Name") {
                println!("  LOG: {}", line.trim());
            }
        }
    }

    if code == 0 {
        let _ = Command::new("msiexec.exe")
            .args(&["/x", product_code, "/qn"])
            .status();
    }
}
