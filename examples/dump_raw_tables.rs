/// Dump table data from our MSI by reading raw streams via cfb.
/// Uses velocity_msi's encode_stream_name to find the right streams.
use std::io::{Cursor, Read};
use std::collections::BTreeMap;

fn main() {
    let msi_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\diag_custom.msi";
    let data = std::fs::read(msi_path).unwrap();
    let cursor = Cursor::new(&data);
    let mut comp = cfb::CompoundFile::open(cursor).unwrap();

    // Read string pool streams (known encoded names)
    let pool_name = velocity_msi::encode_stream_name("_StringPool", true);
    let data_name = velocity_msi::encode_stream_name("_StringData", true);
    
    let pool_path = format!("/{}", pool_name);
    let data_path = format!("/{}", data_name);
    
    let pool_data = read_stream(&mut comp, &pool_path);
    let string_data_bytes = read_stream(&mut comp, &data_path);
    
    let strings = decode_string_pool(&pool_data, &string_data_bytes);
    eprintln!("=== String Pool ({} entries, codepage={}) ===", strings.len(),
        u32::from_le_bytes([pool_data[0], pool_data[1], pool_data[2], pool_data[3]]) & 0xFFFF);
    for (id, s) in &strings {
        eprintln!("  [{}] '{}'", id, s);
    }

    // Read key tables
    for table_name in &["_Tables", "_Columns", "Property", "Directory", "Component",
                        "File", "Media", "Feature", "FeatureComponents",
                        "InstallExecuteSequence", "InstallUISequence"] {
        let enc = velocity_msi::encode_stream_name(table_name, true);
        let path = format!("/{}", enc);
        match try_read_stream(&mut comp, &path) {
            Some(data) => {
                eprintln!("\n=== {} ({} bytes) ===", table_name, data.len());
                eprintln!("  hex: {:02x?}", &data[..data.len().min(128)]);
                
                // Decode for specific tables
                match *table_name {
                    "_Tables" => decode_tables_table(&data, &strings),
                    "FeatureComponents" => decode_fc_table(&data, &strings),
                    "Property" => decode_property_table(&data, &strings),
                    _ => {}
                }
            }
            None => eprintln!("\n=== {} NOT FOUND ===", table_name),
        }
    }
}

fn decode_tables_table(data: &[u8], strings: &BTreeMap<usize, String>) {
    let count = data.len() / 2;
    eprintln!("  {} table names:", count);
    for i in 0..count {
        let id = u16::from_le_bytes([data[i*2], data[i*2+1]]) as usize;
        let name = strings.get(&id).cloned().unwrap_or_else(|| format!("?{}", id));
        eprintln!("    [{}] {}", id, name);
    }
}

fn decode_fc_table(data: &[u8], strings: &BTreeMap<usize, String>) {
    let row_size = 4; // 2 string refs
    let count = data.len() / row_size;
    eprintln!("  {} rows:", count);
    for i in 0..count {
        let off = i * row_size;
        let feat_id = u16::from_le_bytes([data[off], data[off+1]]) as usize;
        let comp_id = u16::from_le_bytes([data[off+2], data[off+3]]) as usize;
        let feat = strings.get(&feat_id).cloned().unwrap_or_else(|| format!("?{}", feat_id));
        let comp_name = strings.get(&comp_id).cloned().unwrap_or_else(|| format!("?{}", comp_id));
        eprintln!("    Feature='{}' [{}] Component='{}' [{}]", feat, feat_id, comp_name, comp_id);
    }
}

fn decode_property_table(data: &[u8], strings: &BTreeMap<usize, String>) {
    // Property: 2 columns (Property string, Value string), column-major
    // Need to know row count. Let's figure it out: data.len() / 4 bytes per row (2 + 2)
    let row_count = data.len() / 4;
    eprintln!("  {} rows (column-major):", row_count);
    for i in 0..row_count {
        // Column 1: Property name (all values first)
        let name_id = u16::from_le_bytes([data[i*2], data[i*2+1]]) as usize;
        // Column 2: Value (after all column 1 values)
        let val_id = u16::from_le_bytes([data[row_count*2 + i*2], data[row_count*2 + i*2+1]]) as usize;
        let name = strings.get(&name_id).cloned().unwrap_or_else(|| format!("?{}", name_id));
        let val = strings.get(&val_id).cloned().unwrap_or_else(|| format!("?{}", val_id));
        eprintln!("    {} = {} [ids: {}, {}]", name, val, name_id, val_id);
    }
}

fn read_stream(comp: &mut cfb::CompoundFile<Cursor<&Vec<u8>>>, path: &str) -> Vec<u8> {
    let mut stream = comp.open_stream(path).unwrap();
    let mut data = Vec::new();
    stream.read_to_end(&mut data).unwrap();
    data
}

fn try_read_stream(comp: &mut cfb::CompoundFile<Cursor<&Vec<u8>>>, path: &str) -> Option<Vec<u8>> {
    match comp.open_stream(path) {
        Ok(mut stream) => {
            let mut data = Vec::new();
            stream.read_to_end(&mut data).ok()?;
            Some(data)
        }
        Err(_) => None,
    }
}

fn decode_string_pool(pool_data: &[u8], string_data: &[u8]) -> BTreeMap<usize, String> {
    let mut strings = BTreeMap::new();
    if pool_data.len() < 4 { return strings; }
    
    let mut offset = 4;
    let mut data_offset = 0usize;
    let mut id = 1usize;
    
    while offset + 4 <= pool_data.len() {
        let length = u16::from_le_bytes([pool_data[offset], pool_data[offset+1]]) as usize;
        let _refcount = u16::from_le_bytes([pool_data[offset+2], pool_data[offset+3]]);
        offset += 4;
        
        if length > 0 && data_offset + length <= string_data.len() {
            let bytes = &string_data[data_offset..data_offset + length];
            let s = String::from_utf8_lossy(bytes).to_string();
            strings.insert(id, s);
            data_offset += length;
        } else if length == 0 {
            // Empty string entry
        }
        id += 1;
    }
    
    strings
}
