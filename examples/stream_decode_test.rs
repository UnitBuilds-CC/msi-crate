/// Decode velocity-msi stream names to identify all tables
/// cargo run --example stream_decode_test -p velocity-msi
use std::io::Cursor;
use velocity_msi::{Column, MsiBuilder, Value};

fn main() {
    println!("=== STREAM DECODE TEST ===\n");

    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_template("Intel", 1033);

    // Create all tables
    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
        vec![Value::from("ProductCode"), Value::from("{12345678-1234-4234-8234-123456789012}")],
        vec![Value::from("UpgradeCode"), Value::from("{22345678-1234-4234-8234-123456789012}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").nullable().string(72).build(),
        Column::build("DefaultDir").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("ProgramFilesFolder"), Value::from("TARGETDIR"), Value::from("PFiles")],
        vec![Value::from("INSTALLDIR"), Value::from("ProgramFilesFolder"), Value::from("Test")],
    ]).unwrap();

    builder.create_table("Component", vec![
        Column::build("Component").string(72).primary_key().build(),
        Column::build("ComponentId").nullable().string(38).build(),
        Column::build("Directory_").string(72).build(),
        Column::build("Attributes").int16().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("KeyPath").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Component", vec![
        vec![Value::from("C1"), Value::Null, Value::from("INSTALLDIR"), Value::Int(0), Value::Null, Value::Null],
    ]).unwrap();

    builder.create_table("Feature", vec![
        Column::build("Feature").string(38).primary_key().build(),
        Column::build("Feature_Parent").nullable().string(38).build(),
        Column::build("Title").nullable().string(64).localizable().build(),
        Column::build("Description").nullable().string(255).localizable().build(),
        Column::build("Display").nullable().int16().build(),
        Column::build("Level").int16().build(),
        Column::build("Directory_").nullable().string(72).build(),
        Column::build("Attributes").int16().build(),
    ]).unwrap();
    builder.insert_rows("Feature", vec![
        vec![Value::from("F1"), Value::Null, Value::Null, Value::Null, Value::Null, Value::Int(1), Value::Null, Value::Int(0)],
    ]).unwrap();

    builder.create_table("FeatureComponents", vec![
        Column::build("Feature_").string(38).primary_key().build(),
        Column::build("Component_").string(72).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("FeatureComponents", vec![
        vec![Value::from("F1"), Value::from("C1")],
    ]).unwrap();

    builder.create_table("InstallExecuteSequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallExecuteSequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![Value::from("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![Value::from("InstallFinalize"), Value::Null, Value::Int(6600)],
    ]).unwrap();

    builder.create_table("InstallUISequence", vec![
        Column::build("Action").string(72).primary_key().build(),
        Column::build("Condition").nullable().string(255).build(),
        Column::build("Sequence").nullable().int16().build(),
    ]).unwrap();
    builder.insert_rows("InstallUISequence", vec![
        vec![Value::from("CostInitialize"), Value::Null, Value::Int(800)],
        vec![Value::from("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![Value::from("ExecuteAction"), Value::Null, Value::Int(1300)],
    ]).unwrap();

    builder.create_table("Media", vec![
        Column::build("DiskId").int16().primary_key().build(),
        Column::build("LastSequence").int16().build(),
        Column::build("DiskPrompt").nullable().string(64).localizable().build(),
        Column::build("Cabinet").nullable().string(255).build(),
        Column::build("VolumeLabel").nullable().string(32).localizable().build(),
        Column::build("Source").nullable().string(72).build(),
    ]).unwrap();
    builder.insert_rows("Media", vec![
        vec![Value::Int(1), Value::Int(0), Value::Null, Value::Null, Value::Null, Value::Null],
    ]).unwrap();

    let msi_data = builder.build().unwrap();
    let mut comp = cfb::CompoundFile::open(Cursor::new(&msi_data)).unwrap();

    println!("velocity-msi streams:\n");
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.name().to_string())
        .collect();

    for name in &stream_names {
        let mut s = comp.open_stream(name).unwrap();
        let mut data = Vec::new();
        std::io::Read::read_to_end(&mut s, &mut data).unwrap();

        let decoded = decode_stream_name(name);
        let hex: String = name.encode_utf16()
            .map(|c| format!("{:04X}", c))
            .collect::<Vec<_>>().join(" ");
        println!("  {} -> {:?} ({} bytes)", hex, decoded, data.len());
    }

    println!("\nTotal: {} streams", stream_names.len());
    println!("\n=== DONE ===");
}

fn decode_stream_name(encoded: &str) -> String {
    let mut chars = encoded.chars().peekable();
    let first = chars.next().unwrap();

    let is_system = first == '\u{4840}';
    let mut result = String::new();

    if is_system {
        // Remaining chars are encoded pairs
        let remaining: Vec<char> = chars.collect();
        let mut i = 0;
        while i < remaining.len() {
            let ch = remaining[i];
            let val = (ch as u32).wrapping_sub(0x3800);
            if i + 1 < remaining.len() {
                let d1 = val % 64;
                let d2 = val / 64;
                if let Some(c1) = to_ascii(d1) { result.push(c1); }
                if let Some(c2) = to_ascii(d2) { result.push(c2); }
                i += 2;
            } else {
                // Single char at end
                let val2 = (ch as u32).wrapping_sub(0x4800);
                if let Some(c1) = to_ascii(val2) { result.push(c1); }
                i += 1;
            }
        }
        format!("[sys]{}", result)
    } else {
        // Non-system: first char might be \u{0005} for SummaryInfo
        if first == '\u{0005}' {
            result.push('\\');
            result.push('x');
            result.push('0');
            result.push('5');
        } else {
            result.push(first);
        }
        for ch in chars {
            result.push(ch);
        }
        result
    }
}

fn to_ascii(v: u32) -> Option<char> {
    if v < 10 { char::from_u32(v + b'0' as u32) }
    else if v < 36 { char::from_u32(v - 10 + b'A' as u32) }
    else if v < 62 { char::from_u32(v - 36 + b'a' as u32) }
    else if v == 62 { Some('.') }
    else if v == 63 { Some('_') }
    else { None }
}
