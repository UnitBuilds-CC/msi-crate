/// Binary compare: use msi crate to read velocity-msi output, then compare stream-by-stream
/// cargo run --example binary_compare_test -p velocity-msi
use std::io::{Cursor, Read};

fn main() {
    println!("=== BINARY COMPARE TEST ===\n");

    // Build velocity-msi with just Property table (simplest case)
    let v_data = build_velocity_simple();
    let m_data = build_msi_crate_simple();

    println!("velocity-msi: {} bytes (V{})", v_data.len(), v_data[26]);
    println!("msi crate:    {} bytes (V{})", m_data.len(), m_data[26]);

    // Try to open velocity-msi with msi crate
    println!("\n--- Opening velocity-msi with msi crate ---");
    match msi::Package::open(Cursor::new(&v_data)) {
        Ok(_pkg) => {
            println!("SUCCESS: msi crate can read velocity-msi output!");
        }
        Err(e) => println!("FAILED: {}", e),
    }

    // Compare SummaryInfo streams
    println!("\n--- SummaryInfo comparison ---");
    {
        let mut v_comp = cfb::CompoundFile::open(Cursor::new(&v_data)).unwrap();
        let mut v_si = v_comp.open_stream("\u{0005}SummaryInformation").unwrap();
        let mut v_si_data = Vec::new();
        v_si.read_to_end(&mut v_si_data).unwrap();

        let mut m_comp = cfb::CompoundFile::open(Cursor::new(&m_data)).unwrap();
        let mut m_si = m_comp.open_stream("\u{0005}SummaryInformation").unwrap();
        let mut m_si_data = Vec::new();
        m_si.read_to_end(&mut m_si_data).unwrap();

        println!("velocity-msi SummaryInfo: {} bytes", v_si_data.len());
        println!("msi crate SummaryInfo:    {} bytes", m_si_data.len());

        // Compare byte by byte
        let min_len = v_si_data.len().min(m_si_data.len());
        let mut diffs = 0;
        for i in 0..min_len {
            if v_si_data[i] != m_si_data[i] {
                if diffs < 20 {
                    println!("  Diff at offset {}: velocity=0x{:02X}, msi=0x{:02X}", i, v_si_data[i], m_si_data[i]);
                }
                diffs += 1;
            }
        }
        if v_si_data.len() != m_si_data.len() {
            println!("  Size difference: {} vs {} bytes", v_si_data.len(), m_si_data.len());
        }
        println!("  Total byte differences: {}", diffs);
    }

    // Compare _StringPool streams
    println!("\n--- _StringPool comparison ---");
    compare_stream(&v_data, &m_data, "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3B6A}\u{45E4}\u{4824}", "_StringPool");

    // Compare _StringData streams
    println!("\n--- _StringData comparison ---");
    compare_stream(&v_data, &m_data, "\u{4840}\u{3F3F}\u{4577}\u{446C}\u{3E6A}\u{44B2}\u{482F}", "_StringData");

    // Compare _Columns streams
    println!("\n--- _Columns comparison ---");
    compare_stream(&v_data, &m_data, "\u{4840}\u{3B3F}\u{43F2}\u{4438}\u{45B1}", "_Columns");

    // Compare _Tables streams
    println!("\n--- _Tables comparison ---");
    compare_stream(&v_data, &m_data, "\u{4840}\u{3F7F}\u{4164}\u{422F}\u{4836}", "_Tables");

    // Compare Property table streams
    println!("\n--- Property comparison ---");
    compare_stream(&v_data, &m_data, "\u{4840}\u{4559}\u{44F2}\u{4568}\u{4737}", "Property");

    println!("\n=== DONE ===");
}

fn compare_stream(v_data: &[u8], m_data: &[u8], encoded_name: &str, label: &str) {
    let mut v_comp = cfb::CompoundFile::open(Cursor::new(v_data)).unwrap();
    let mut m_comp = cfb::CompoundFile::open(Cursor::new(m_data)).unwrap();

    let v_result = v_comp.open_stream(encoded_name);
    let m_result = m_comp.open_stream(encoded_name);

    match (v_result, m_result) {
        (Ok(mut vs), Ok(mut ms)) => {
            let mut v_bytes = Vec::new();
            vs.read_to_end(&mut v_bytes).unwrap();
            let mut m_bytes = Vec::new();
            ms.read_to_end(&mut m_bytes).unwrap();

            println!("velocity-msi {}: {} bytes", label, v_bytes.len());
            println!("msi crate {}:    {} bytes", label, m_bytes.len());

            let min_len = v_bytes.len().min(m_bytes.len());
            let mut diffs = 0;
            for i in 0..min_len {
                if v_bytes[i] != m_bytes[i] {
                    if diffs < 10 {
                        println!("  Diff at {}: v=0x{:02X} m=0x{:02X}", i, v_bytes[i], m_bytes[i]);
                    }
                    diffs += 1;
                }
            }
            if v_bytes.len() != m_bytes.len() {
                println!("  Size diff: {} vs {}", v_bytes.len(), m_bytes.len());
            }
            println!("  Total diffs: {}", diffs);

            // Show first 64 bytes of each for comparison
            println!("  velocity first 64: {:02x?}", &v_bytes[..v_bytes.len().min(64)]);
            println!("  msi crate first 64: {:02x?}", &m_bytes[..m_bytes.len().min(64)]);
        }
        (Err(e), _) => println!("velocity-msi doesn't have {}: {}", label, e),
        (_, Err(e)) => println!("msi crate doesn't have {}: {}", label, e),
    }
}

fn build_velocity_simple() -> Vec<u8> {
    use velocity_msi::{Column, MsiBuilder, Value};
    let mut builder = MsiBuilder::new();
    builder.set_title("Test");
    builder.set_template("Intel", 1033);

    builder.create_table("Property", vec![
        Column::build("Property").string(72).primary_key().build(),
        Column::build("Value").nullable().string(255).localizable().build(),
    ]).unwrap();
    builder.insert_rows("Property", vec![
        vec![Value::from("ProductName"), Value::from("Test")],
        vec![Value::from("ProductVersion"), Value::from("1.0")],
        vec![Value::from("Manufacturer"), Value::from("Test")],
    ]).unwrap();

    builder.build().unwrap()
}

fn build_msi_crate_simple() -> Vec<u8> {
    let cursor = Cursor::new(Vec::new());
    let mut pkg = msi::Package::create(msi::PackageType::Installer, cursor).unwrap();
    {
        let si = pkg.summary_info_mut();
        si.set_title("Test");
        si.set_codepage(msi::CodePage::Windows1252);
        si.set_arch("Intel");
        si.set_languages(&[msi::Language::from_code(1033)]);
        si.set_word_count(2);
        si.set_uuid(uuid::Uuid::nil());
    }
    pkg.set_database_codepage(msi::CodePage::Windows1252);

    pkg.create_table("Property", vec![
        msi::Column::build("Property").primary_key().id_string(72),
        msi::Column::build("Value").nullable().localizable().formatted_string(255),
    ]).unwrap();
    pkg.insert_rows(msi::Insert::into("Property")
        .row(vec![msi::Value::Str("ProductName".into()), msi::Value::Str("Test".into())])
        .row(vec![msi::Value::Str("ProductVersion".into()), msi::Value::Str("1.0".into())])
        .row(vec![msi::Value::Str("Manufacturer".into()), msi::Value::Str("Test".into())])
    ).unwrap();

    pkg.flush().unwrap();
    pkg.into_inner().unwrap().into_inner()
}
