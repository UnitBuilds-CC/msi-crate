/// Extract raw streams from our MSI by parsing the OLE structure manually.
/// This avoids cfb borrow issues by working directly with bytes.
use velocity_msi::{MsiBuilder, Column, Value, encode_stream_name};

fn main() {
    // Build the failing MSI
    let mut b = MsiBuilder::new();
    b.set_title("BinDiag");
    b.set_author("V");
    b.set_template("Intel", 1033);

    b.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").string(255).nullable().build(),
    ]).unwrap();
    b.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("BinDiag")],
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
    std::fs::write("bindiag.msi", &msi_data).unwrap();
    
    // Use cfb to extract streams - collect names first, then read
    let stream_names: Vec<String> = {
        let cursor = std::io::Cursor::new(&msi_data);
        let comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        comp.walk().filter(|e| e.is_stream()).map(|e| e.name().to_string()).collect()
    };
    
    println!("=== Streams in our MSI ===");
    for name in &stream_names {
        let cursor = std::io::Cursor::new(&msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        let path = std::path::Path::new(name);
        let mut data = Vec::new();
        let mut stream = comp.open_stream(path).expect("open stream");
        std::io::Read::read_to_end(&mut stream, &mut data).expect("read stream");
        
        println!("\nStream: {:?} ({} bytes)", name, data.len());
        if data.len() <= 128 {
            print!("  ");
            for (i, b) in data.iter().enumerate() {
                if i > 0 && i % 16 == 0 { print!("\n  "); }
                print!("{:02X} ", b);
            }
            println!();
        } else {
            print!("  ");
            for (i, b) in data[..128].iter().enumerate() {
                if i > 0 && i % 16 == 0 { print!("\n  "); }
                print!("{:02X} ", b);
            }
            println!("\n  ... ({} more bytes)", data.len() - 128);
        }
    }
    
    // Now specifically decode the string pool
    println!("\n\n=== String Pool Decode ===");
    let pool_enc = encode_stream_name("_StringPool", true);
    let data_enc = encode_stream_name("_StringData", true);
    
    let pool_data = {
        let cursor = std::io::Cursor::new(&msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        let mut d = Vec::new();
        let mut s = comp.open_stream(std::path::Path::new(&pool_enc)).expect("open pool");
        std::io::Read::read_to_end(&mut s, &mut d).expect("read pool");
        d
    };
    
    let string_data = {
        let cursor = std::io::Cursor::new(&msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        let mut d = Vec::new();
        let mut s = comp.open_stream(std::path::Path::new(&data_enc)).expect("open data");
        std::io::Read::read_to_end(&mut s, &mut d).expect("read data");
        d
    };
    
    // Parse pool header
    let header = u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]);
    let codepage = header & 0xFFFF;
    println!("Codepage: {}, Long refs: {}", codepage, (header & 0x80000000) != 0);
    
    // Parse pool entries
    let mut offset = 4;
    let mut id = 1u16;
    let mut string_offsets = Vec::new(); // (start, len) for each ID
    let mut data_offset = 0usize;
    while offset + 4 <= pool_data.len() {
        let len = u16::from_le_bytes([pool_data[offset], pool_data[offset+1]]) as usize;
        let refcount = u16::from_le_bytes([pool_data[offset+2], pool_data[offset+3]]);
        
        let text = if data_offset + len <= string_data.len() {
            String::from_utf8_lossy(&string_data[data_offset..data_offset+len]).to_string()
        } else {
            "OUT OF BOUNDS".to_string()
        };
        
        println!("  ID {}: len={}, refcount={}, text={:?}", id, len, refcount, text);
        string_offsets.push((data_offset, len));
        data_offset += len;
        offset += 4;
        id += 1;
    }
    println!("Total pool entries: {}", id - 1);
    println!("StringData consumed: {} / {} bytes", data_offset, string_data.len());
    
    // Now decode Directory stream using pool IDs
    let dir_enc = encode_stream_name("Directory", true);
    let dir_data = {
        let cursor = std::io::Cursor::new(&msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        let mut d = Vec::new();
        let mut s = comp.open_stream(std::path::Path::new(&dir_enc)).expect("open dir");
        std::io::Read::read_to_end(&mut s, &mut d).expect("read dir");
        d
    };
    
    println!("\n=== Directory Stream Decode ===");
    println!("Bytes: {}", hex_dump(&dir_data));
    // Column-major: col0 (3 rows? no, 1 row × 3 cols)
    // col0: 1 × u16 = 2 bytes (Directory)
    // col1: 1 × u16 = 2 bytes (Directory_Parent)
    // col2: 1 × u16 = 2 bytes (DefaultDir)
    if dir_data.len() >= 6 {
        let col0_id = u16::from_le_bytes([dir_data[0], dir_data[1]]);
        let col1_id = u16::from_le_bytes([dir_data[2], dir_data[3]]);
        let col2_id = u16::from_le_bytes([dir_data[4], dir_data[5]]);
        
        let resolve = |id: u16| -> String {
            if id == 0 { return "NULL".to_string(); }
            let idx = (id - 1) as usize;
            if idx < string_offsets.len() {
                let (start, len) = string_offsets[idx];
                String::from_utf8_lossy(&string_data[start..start+len]).to_string()
            } else {
                format!("INVALID_ID_{}", id)
            }
        };
        
        println!("  Directory = pool_id {} = {:?}", col0_id, resolve(col0_id));
        println!("  Directory_Parent = pool_id {} = {:?}", col1_id, resolve(col1_id));
        println!("  DefaultDir = pool_id {} = {:?}", col2_id, resolve(col2_id));
    }
    
    // Decode _Columns stream
    let cols_enc = encode_stream_name("_Columns", true);
    let cols_data = {
        let cursor = std::io::Cursor::new(&msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("cfb open");
        let mut d = Vec::new();
        let mut s = comp.open_stream(std::path::Path::new(&cols_enc)).expect("open cols");
        std::io::Read::read_to_end(&mut s, &mut d).expect("read cols");
        d
    };
    
    println!("\n=== _Columns Stream Decode ===");
    let row_count = cols_data.len() / 8;
    println!("{} rows ({} bytes)", row_count, cols_data.len());
    
    let resolve = |id: u16| -> String {
        if id == 0 { return "NULL".to_string(); }
        let idx = (id - 1) as usize;
        if idx < string_offsets.len() {
            let (start, len) = string_offsets[idx];
            String::from_utf8_lossy(&string_data[start..start+len]).to_string()
        } else {
            format!("INVALID_{}", id)
        }
    };
    
    // Column-major: col0(row_count × u16), col1(row_count × u16), col2(row_count × u16), col3(row_count × u16)
    let col0_start = 0;
    let col1_start = row_count * 2;
    let col2_start = row_count * 4;
    let col3_start = row_count * 6;
    
    for i in 0..row_count {
        let table_id = u16::from_le_bytes([cols_data[col0_start + i*2], cols_data[col0_start + i*2 + 1]]);
        let number_raw = i16::from_le_bytes([cols_data[col1_start + i*2], cols_data[col1_start + i*2 + 1]]);
        let number = number_raw ^ (-0x8000i16);
        let name_id = u16::from_le_bytes([cols_data[col2_start + i*2], cols_data[col2_start + i*2 + 1]]);
        let type_raw = i16::from_le_bytes([cols_data[col3_start + i*2], cols_data[col3_start + i*2 + 1]]);
        let type_val = (type_raw ^ (-0x8000i16)) as u16;
        
        println!("  Row {}: Table={}({}), Number={}, Name={}({}), Type=0x{:04X}",
            i, table_id, resolve(table_id), number, name_id, resolve(name_id), type_val);
    }
}

fn hex_dump(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}
