use cfb::CompoundFile;
use std::io::{Cursor, Read, Write};
use std::path::Path;

fn main() {
    let cursor = Cursor::new(Vec::new());
    let mut comp = CompoundFile::create_with_version(cfb::Version::V4, cursor).unwrap();
    let clsid = uuid::Uuid::parse_str("000c1084-0000-0000-c000-000000000046").unwrap();
    comp.set_storage_clsid("/", clsid).unwrap();
    
    // Write the system MSI's SummaryInformation
    let sys_file = std::fs::File::open("C:\\Windows\\Installer\\10d16cbb.msi").unwrap();
    let mut sys_comp = CompoundFile::open(sys_file).unwrap();
    let si_path = Path::new("\u{0005}SummaryInformation");
    let mut sys_si = sys_comp.open_stream(si_path).unwrap();
    let mut si_data = Vec::new();
    sys_si.read_to_end(&mut si_data).unwrap();
    
    let mut stream = comp.create_stream("\u{0005}SummaryInformation").unwrap();
    stream.write_all(&si_data).unwrap();
    drop(stream);
    
    let cursor = comp.into_inner();
    let data = cursor.into_inner();
    std::fs::write("target/minimal_msi.msi", &data).unwrap();
    println!("Created target/minimal_msi.msi ({} bytes)", data.len());
}
