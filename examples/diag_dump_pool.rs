/// Dump string pool and _Columns data from our MSI to find the bug
/// cargo run --example diag_dump_pool -p velocity-msi
use velocity_msi::{Column, MsiBuilder, Value};
use std::io::Cursor;

fn make_uuid() -> String {
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!(
        "{{{:08X}-{:04X}-4{:03X}-{:04X}-{:08X}{:04X}}}",
        (t & 0xFFFFFFFF) as u32,
        ((t >> 32) & 0xFFFF) as u16,
        ((t >> 48) & 0x0FFF) as u16,
        (((t >> 44) & 0x0FFF) as u16) | 0x8000,
        ((t >> 16) & 0xFFFFFFFF) as u32,
        (t & 0xFFFF) as u16,
    )
}

fn main() {
    let pc = make_uuid();
    let uc = make_uuid();
    let mut b = MsiBuilder::new();
    b.set_title("Test");
    b.set_author("V");
    b.set_template("Intel", 1033);
    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0.0")],
        vec![Value::from("Manufacturer"), Value::from("V")],
        vec![Value::from("ProductCode"), Value::from(pc.as_str())],
        vec![Value::from("UpgradeCode"), Value::from(uc.as_str())],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();
    b.create_table("File", vec![
        Column::build("File_").string(72).primary_key().build(),
        Column::build("Component_").string(72).build(),
        Column::build("FileName").string(255).localizable().build(),
        Column::build("FileSize").int32().build(),
        Column::build("Attributes").nullable().int16().build(),
        Column::build("Sequence").int16().build(),
    ]).unwrap();
    b.insert_rows("File", vec![
        vec![Value::from("F1"), Value::from("MC"), Value::from("test.txt"),
             Value::Int(10), Value::Int(0), Value::Int(1)],
    ]).unwrap();
    let data = b.build().unwrap();

    // Read back with msi crate
    let cursor = Cursor::new(&data);
    let mut package = msi::Package::open(cursor).unwrap();

    // Dump _Columns for File table
    println!("=== _Columns for File (read by msi crate) ===");
    let rows = package.select_rows(msi::Select::table("_Columns")).unwrap();
    for row in rows {
        if let msi::Value::Str(ref t) = row["Table"] {
            if t == "File" {
                let num = match row["Number"] { msi::Value::Int(n) => n, _ => 0 };
                let name = match row["Name"] { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
                let typ = match row["Type"] { msi::Value::Int(n) => n as u32, _ => 0 };
                println!("  Number={} Name='{}' Type=0x{:04X}", num, name, typ);
            }
        }
    }

    // Dump ALL _Columns entries
    println!("\n=== ALL _Columns entries ===");
    let rows = package.select_rows(msi::Select::table("_Columns")).unwrap();
    for row in rows {
        let table = match row["Table"] { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
        let num = match row["Number"] { msi::Value::Int(n) => n, _ => 0 };
        let name = match row["Name"] { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
        let typ = match row["Type"] { msi::Value::Int(n) => n as u32, _ => 0 };
        println!("  Table='{}' Number={} Name='{}' Type=0x{:04X}", table, num, name, typ);
    }

    // Dump _Tables
    println!("\n=== _Tables entries ===");
    let rows = package.select_rows(msi::Select::table("_Tables")).unwrap();
    for row in rows {
        let name = match row["Name"] { msi::Value::Str(ref s) => s.clone(), _ => String::new() };
        println!("  Name='{}'", name);
    }

    // Dump string pool by reading raw _StringData
    println!("\n=== All streams ===");
    for name in package.streams() {
        println!("  {:?}", name);
    }

    // Read raw _StringPool and _StringData from the OLE file
    println!("\n=== Raw string pool analysis ===");
    let mut comp = cfb::CompoundFile::open(Cursor::new(&data)).unwrap();
    let stream_names: Vec<String> = comp.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_str().unwrap().to_string())
        .collect();

    for sn in &stream_names {
        // Find _StringPool stream (contains "StringPool")
        if sn.contains("\u{4438}\u{45b1}") { // Part of encoded _StringPool
            let mut s = comp.open_stream(sn.as_str()).unwrap();
            let mut buf = Vec::new();
            use std::io::Read;
            s.read_to_end(&mut buf).unwrap();
            println!("_StringPool stream: {} ({} bytes)", sn, buf.len());

            if buf.len() >= 4 {
                let header = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                let codepage = header & 0xFFFF;
                let long_refs = (header & 0x80000000) != 0;
                println!("  Codepage: {}, Long refs: {}", codepage, long_refs);

                let entry_size = 4; // u16 len + u16 refcount
                let num_entries = (buf.len() - 4) / entry_size;
                println!("  Entries: {}", num_entries);

                // Now read _StringData
                for sn2 in &stream_names {
                    if sn2.contains("\u{4438}\u{45b2}") { // Part of encoded _StringData
                        // Actually, let me find it differently
                    }
                }
            }
        }
    }

    // Find and dump _StringData
    for sn in &stream_names {
        // _StringData encoded name contains specific chars
        if sn.len() > 5 && !sn.contains("\u{4840}") && sn.starts_with('/') {
            // Try to read as string data
        }
    }

    // Better approach: read string pool using the velocity-msi StringPool
    println!("\n=== String pool contents (from velocity-msi) ===");
    // We can't access the string pool after build(), so let's use the msi crate

    // Use the msi crate to read all string pool data
    println!("\n=== Reading string pool via msi crate ===");
    // The msi crate reads the string pool when opening the package
    // We can check specific string references by reading table data

    // Read File table data
    println!("\n=== File table data ===");
    let rows = package.select_rows(msi::Select::table("File")).unwrap();
    for row in rows {
        println!("  File_={:?} Component_={:?} FileName={:?} FileSize={:?} Attributes={:?} Sequence={:?}",
            row["File_"], row["Component_"], row["FileName"],
            row["FileSize"], row["Attributes"], row["Sequence"]);
    }

    // Read Property table data
    println!("\n=== Property table data ===");
    let rows = package.select_rows(msi::Select::table("Property")).unwrap();
    for row in rows {
        println!("  {:?} = {:?}", row["Property"], row["Value"]);
    }

    // Check database codepage
    println!("\n=== Database codepage ===");
    println!("  {:?}", package.database_codepage());
}
