use cfb::{CompoundFile, Version};
use std::io::{Cursor, Write};

fn main() {
    // Test with V4 (default)
    let cursor = Cursor::new(Vec::new());
    let mut comp = CompoundFile::create_with_version(Version::V4, cursor).unwrap();
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    comp.set_storage_clsid("/", clsid).unwrap();
    
    // Write a simple stream
    let mut stream = comp.create_stream("\u{0005}SummaryInformation").unwrap();
    // Write minimal property set
    let mut data = Vec::new();
    data.write_all(&0xFFFEu16.to_le_bytes()).unwrap(); // BOM
    data.write_all(&0u16.to_le_bytes()).unwrap(); // version
    data.write_all(&10u16.to_le_bytes()).unwrap(); // OS low
    data.write_all(&2u16.to_le_bytes()).unwrap(); // OS high
    data.write_all(&[0u8; 16]).unwrap(); // CLSID
    data.write_all(&1u32.to_le_bytes()).unwrap(); // reserved
    // FMTID
    let fmtid: [u8; 16] = *b"\xe0\x85\x9f\xf2\xf9\x4f\x68\x10\xab\x91\x08\x00\x2b\x27\xb3\xd9";
    data.write_all(&fmtid).unwrap();
    data.write_all(&48u32.to_le_bytes()).unwrap(); // section offset
    // Section
    let sec_size: u32 = 8 + 8 + 8; // header + 1 prop entry + prop data (VT_I2 = 8 bytes)
    data.write_all(&sec_size.to_le_bytes()).unwrap();
    data.write_all(&1u32.to_le_bytes()).unwrap(); // 1 property
    data.write_all(&1u32.to_le_bytes()).unwrap(); // PID_CODEPAGE
    data.write_all(&24u32.to_le_bytes()).unwrap(); // offset to value (8 + 8 = 16 from section start... wait)
    // Actually offset should be 8 (header) + 8 (1 index entry) = 16
    // Let me fix: offset = 16
    data.truncate(data.len() - 4);
    data.write_all(&16u32.to_le_bytes()).unwrap();
    // Value: VT_I2 codepage 1252
    data.write_all(&2u32.to_le_bytes()).unwrap(); // VT_I2
    data.write_all(&1252i16.to_le_bytes()).unwrap();
    data.write_all(&0u16.to_le_bytes()).unwrap(); // padding
    stream.write_all(&data).unwrap();
    drop(stream);
    
    let cursor = comp.into_inner();
    std::fs::write("target/test_v4.msi", cursor.into_inner()).unwrap();
    println!("Created V4 MSI");
}
