/// Dump string pool and Directory table data to verify internal consistency.
use velocity_msi::{MsiBuilder, Column, Value};
use std::io::Read;

fn main() {
    let mut builder = MsiBuilder::new();
    builder.set_title("Dump Test");
    builder.set_author("Velocity");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(1024).build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Dump Test")],
        vec![Value::from("ProductCode"), Value::from("{AADDD4E4-3A12-4B7D-A4E0-5A2F8B3C6D90}")],
        vec![Value::from("ProductLanguage"), Value::from("1033")],
    ]).unwrap();

    builder.create_table("Directory", vec![
        Column::build("Directory").string(72).primary_key().build(),
        Column::build("Directory_Parent").string(72).nullable().build(),
        Column::build("DefaultDir").string(255).primary_key().build(),
    ]).unwrap();
    builder.insert_rows("Directory", vec![
        vec![Value::from("TARGETDIR"), Value::Null, Value::from("SourceDir")],
        vec![Value::from("INSTALLDIR"), Value::from("TARGETDIR"), Value::from("VelTest:VelTest")],
    ]).unwrap();

    let msi = builder.build().unwrap();
    std::fs::write("dump_test.msi", &msi).unwrap();

    let mut cfb = cfb::CompoundFile::open(std::io::Cursor::new(&msi)).unwrap();

    // Collect stream names first (to avoid borrow conflict)
    let stream_names: Vec<String> = cfb.walk()
        .filter(|e| e.is_stream())
        .map(|e| e.path().to_string_lossy().to_string())
        .collect();
    println!("=== {} Streams ===", stream_names.len());
    for p in &stream_names { println!("  {}", p); }

    // Read string pool
    let pool_name = velocity_msi::encode_stream_name("_StringPool", true);
    let data_name = velocity_msi::encode_stream_name("_StringData", true);

    let pool_data = read_stream(&mut cfb, &pool_name);
    let string_data = read_stream(&mut cfb, &data_name);

    let codepage = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]);
    let long_refs = (codepage & 0x80000000) != 0;
    println!("\n=== String Pool (cp={}, long={}) ===", codepage & 0xFFFF, long_refs);

    let num_strings = (pool_data.len() - 4) / 4;
    let mut strings: Vec<(String, u32)> = Vec::new();
    let mut data_off = 0usize;
    for i in 0..num_strings {
        let base = 4 + i * 4;
        let len = u16::from_le_bytes([pool_data[base], pool_data[base + 1]]) as usize;
        let rc = u16::from_le_bytes([pool_data[base + 2], pool_data[base + 3]]);
        let bytes = &string_data[data_off..data_off + len];
        let text = String::from_utf8_lossy(bytes).to_string();
        let id = (i + 1) as u32;
        println!("  {:3}: {:3}b rc={:3} {:?}", id, len, rc, text);
        strings.push((text, id));
        data_off += len;
    }

    // Decode Directory table
    println!("\n=== Directory Table ===");
    let dir_name = velocity_msi::encode_stream_name("Directory", true);
    let dir_data = read_stream(&mut cfb, &dir_name);
    println!("Size: {} bytes, hex: {:02x?}", dir_data.len(), &dir_data[..dir_data.len().min(48)]);

    if dir_data.len() >= 12 {
        let r = |o: usize| u16::from_le_bytes([dir_data[o], dir_data[o+1]]);
        println!("Row1: Dir={}({:?}) Par={}({:?}) Def={}({:?})",
            r(0), lookup(&strings, r(0)),
            r(4), lookup(&strings, r(4)),
            r(8), lookup(&strings, r(8)));
        println!("Row2: Dir={}({:?}) Par={}({:?}) Def={}({:?})",
            r(2), lookup(&strings, r(2)),
            r(6), lookup(&strings, r(6)),
            r(10), lookup(&strings, r(10)));
    }

    // _Tables
    println!("\n=== _Tables ===");
    let tables_name = velocity_msi::encode_stream_name("_Tables", true);
    let tables_data = read_stream(&mut cfb, &tables_name);
    for i in 0..tables_data.len()/2 {
        let r = u16::from_le_bytes([tables_data[i*2], tables_data[i*2+1]]);
        println!("  {} -> {:?}", r, lookup(&strings, r));
    }

    // _Columns for Directory
    println!("\n=== _Columns (Directory entries) ===");
    let cols_name = velocity_msi::encode_stream_name("_Columns", true);
    let cols_data = read_stream(&mut cfb, &cols_name);
    // _Columns: Table(str), Number(int16), Name(str), Type(int16) = 2+2+2+2 = 8 bytes per row
    // Column-major: all Tables, all Numbers, all Names, all Types
    let total_rows = cols_data.len() / 8;
    println!("Total _Columns rows: {}", total_rows);
    for i in 0..total_rows {
        let tbl_ref = u16::from_le_bytes([cols_data[i*2], cols_data[i*2+1]]);
        let num_off = total_rows * 2 + i * 2;
        let num = i16::from_le_bytes([cols_data[num_off], cols_data[num_off+1]]);
        let num_xor = num ^ -0x8000;
        let name_off = total_rows * 4 + i * 2;
        let name_ref = u16::from_le_bytes([cols_data[name_off], cols_data[name_off+1]]);
        let type_off = total_rows * 6 + i * 2;
        let typ = i16::from_le_bytes([cols_data[type_off], cols_data[type_off+1]]);
        let typ_xor = typ ^ -0x8000;
        let tbl_name = lookup(&strings, tbl_ref);
        if tbl_name == "Directory" || tbl_name.starts_with("_") {
            println!("  Table={:?}({}) Num={} Name={:?}({}) Type=0x{:04x}",
                tbl_name, tbl_ref, num_xor, lookup(&strings, name_ref), name_ref, typ_xor as u16);
        }
    }

    // msiexec test
    println!("\n=== msiexec ===");
    let _ = std::fs::remove_dir_all("C:\\VelTest");
    let output = std::process::Command::new("msiexec")
        .args(&["/i", "dump_test.msi", "/qn", "/l*v", "dump_log.txt"])
        .output().unwrap();
    println!("exit: {}", output.status.code().unwrap_or(-1));
    if let Ok(log) = std::fs::read_to_string("dump_log.txt") {
        for line in log.lines() {
            if (line.contains("Error ") && !line.contains("Error 0")) || line.contains("return value 3") {
                println!("  {}", line.trim());
            }
        }
    }
}

fn read_stream(cfb: &mut cfb::CompoundFile<std::io::Cursor<&Vec<u8>>>, path: &str) -> Vec<u8> {
    let mut s = cfb.open_stream(path).unwrap();
    let mut d = Vec::new();
    s.read_to_end(&mut d).unwrap();
    d
}

fn lookup(strings: &[(String, u32)], id: u16) -> String {
    if id == 0 { return "NULL".into(); }
    strings.iter().find(|(_, s)| *s == id as u32)
        .map(|(t, _)| t.clone())
        .unwrap_or_else(|| format!("?{}", id))
}
