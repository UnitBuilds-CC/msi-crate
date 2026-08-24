/// Definitive test: Build MSI with msi crate + cab crate, test with msiexec.
/// If this MSI works with ProcessComponents but velocity-msi doesn't,
/// the bug is in our OLE/serialization layer.
use std::io::{Cursor, Write};
use msi::{Package, PackageType, Insert, Value, Column, CodePage, Language};

fn main() {
    eprintln!("=== Building reference MSI with msi crate ===");
    
    let cursor = Cursor::new(Vec::new());
    let mut pkg: Package<Cursor<Vec<u8>>> = Package::create(PackageType::Installer, cursor)
        .expect("create package");
    
    pkg.set_database_codepage(CodePage::Windows1252);
    
    let si = pkg.summary_info_mut();
    si.set_title("Reference MSI");
    si.set_subject("Test");
    si.set_author("Velocity");
    si.set_arch("x64");
    si.set_languages(&[Language::from_code(1033)]);
    si.set_comments("Test");
    si.set_creating_application("Velocity Installer");
    si.set_uuid(uuid::Uuid::parse_str("AABBCCDD-1234-5678-9ABC-DEF012345678").unwrap());
    si.set_word_count(2); // PID 15 = 2 (required for standard installer)
    
    // Create tables
    pkg.create_table("Property", vec![
        Column::build("Property").primary_key().string(72),
        Column::build("Value").nullable().localizable().string(255),
    ]).unwrap();
    
    pkg.create_table("Directory", vec![
        Column::build("Directory").primary_key().string(72),
        Column::build("Directory_Parent").nullable().string(72),
        Column::build("DefaultDir").localizable().string(255),
    ]).unwrap();
    
    pkg.create_table("Component", vec![
        Column::build("Component").primary_key().string(72),
        Column::build("ComponentId").nullable().string(38),
        Column::build("Directory_").string(72),
        Column::build("Attributes").int16(),
        Column::build("Condition").nullable().string(255),
        Column::build("KeyPath").nullable().string(72),
    ]).unwrap();
    
    pkg.create_table("File", vec![
        Column::build("File").primary_key().string(72),
        Column::build("Component_").string(72),
        Column::build("FileName").localizable().string(255),
        Column::build("FileSize").int32(),
        Column::build("Version").nullable().string(72),
        Column::build("Language").nullable().string(20),
        Column::build("Attributes").nullable().int16(),
        Column::build("Sequence").int16(),
    ]).unwrap();
    
    pkg.create_table("Feature", vec![
        Column::build("Feature").primary_key().string(38),
        Column::build("Feature_Parent").nullable().string(38),
        Column::build("Title").nullable().string(64),
        Column::build("Description").nullable().string(255),
        Column::build("Display").nullable().int16(),
        Column::build("Level").int16(),
        Column::build("Directory_").nullable().string(72),
        Column::build("Attributes").nullable().int16(),
    ]).unwrap();
    
    pkg.create_table("FeatureComponents", vec![
        Column::build("Feature_").primary_key().string(38),
        Column::build("Component_").primary_key().string(72),
    ]).unwrap();
    
    pkg.create_table("Media", vec![
        Column::build("DiskId").primary_key().int16(),
        Column::build("LastSequence").int16(),
        Column::build("Cabinet").nullable().string(255),
        Column::build("VolumeLabel").nullable().string(32),
        Column::build("Source").nullable().string(72),
    ]).unwrap();
    
    pkg.create_table("InstallExecuteSequence", vec![
        Column::build("Action").primary_key().string(72),
        Column::build("Condition").nullable().string(255),
        Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    
    pkg.create_table("InstallUISequence", vec![
        Column::build("Action").primary_key().string(72),
        Column::build("Condition").nullable().string(255),
        Column::build("Sequence").nullable().int16(),
    ]).unwrap();
    
    // Populate
    let product_code = "{AABBCCDD-1234-5678-9ABC-DEF012345678}";
    let upgrade_code = "{BBCCDDEE-2345-6789-ABCD-EF0123456789}";
    
    pkg.insert_rows(Insert::into("Property").rows(vec![
        vec![v("ProductName"), v("Reference Test App")],
        vec![v("ProductVersion"), v("1.0.0")],
        vec![v("Manufacturer"), v("Velocity Test")],
        vec![v("ProductCode"), v(product_code)],
        vec![v("UpgradeCode"), v(upgrade_code)],
        vec![v("ProductLanguage"), v("1033")],
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("Directory").rows(vec![
        vec![v("TARGETDIR"), Value::Null, v("SourceDir")],
        vec![v("ProgramFilesFolder"), v("TARGETDIR"), v("PFiles")],
        vec![v("AppDataFolder"), v("ProgramFilesFolder"), v("RefTest")],
    ])).unwrap();
    
    let file1_data = b"Hello World from reference MSI!\n";
    let file2_data = b"Data file content\n";
    
    pkg.insert_rows(Insert::into("Component").rows(vec![
        vec![v("comp_1"), Value::Null, v("AppDataFolder"), Value::Int(0), Value::Null, v("file_1")],
        vec![v("comp_2"), Value::Null, v("AppDataFolder"), Value::Int(0), Value::Null, v("file_2")],
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("File").rows(vec![
        vec![v("file_1"), v("comp_1"), v("readme.txt"), Value::Int(file1_data.len() as i32),
             Value::Null, Value::Null, Value::Null, Value::Int(1)],
        vec![v("file_2"), v("comp_2"), v("data.txt"), Value::Int(file2_data.len() as i32),
             Value::Null, Value::Null, Value::Null, Value::Int(2)],
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("Feature").row(vec![
        v("Complete"), Value::Null, v("Complete Installation"), v("All features"),
        Value::Int(0), Value::Int(1), v("AppDataFolder"), Value::Null,
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("FeatureComponents").rows(vec![
        vec![v("Complete"), v("comp_1")],
        vec![v("Complete"), v("comp_2")],
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("Media").row(vec![
        Value::Int(1), Value::Int(2), v("#ref.cab"), Value::Null, Value::Null,
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("InstallExecuteSequence").rows(vec![
        vec![v("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![v("FindRelatedProducts"), Value::Null, Value::Int(200)],
        vec![v("CostInitialize"), Value::Null, Value::Int(800)],
        vec![v("FileCost"), Value::Null, Value::Int(900)],
        vec![v("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![v("InstallValidate"), Value::Null, Value::Int(1400)],
        vec![v("InstallInitialize"), Value::Null, Value::Int(1500)],
        vec![v("ProcessComponents"), Value::Null, Value::Int(1700)],
        vec![v("InstallFiles"), Value::Null, Value::Int(4000)],
        vec![v("RegisterProduct"), Value::Null, Value::Int(6100)],
        vec![v("PublishFeatures"), Value::Null, Value::Int(6300)],
        vec![v("PublishProduct"), Value::Null, Value::Int(6400)],
        vec![v("InstallFinalize"), Value::Null, Value::Int(6600)],
    ])).unwrap();
    
    pkg.insert_rows(Insert::into("InstallUISequence").rows(vec![
        vec![v("LaunchConditions"), Value::Null, Value::Int(100)],
        vec![v("FindRelatedProducts"), Value::Null, Value::Int(200)],
        vec![v("CostInitialize"), Value::Null, Value::Int(800)],
        vec![v("FileCost"), Value::Null, Value::Int(900)],
        vec![v("CostFinalize"), Value::Null, Value::Int(1000)],
        vec![v("ExecuteAction"), Value::Null, Value::Int(1300)],
    ])).unwrap();
    
    // Flush
    pkg.flush().expect("flush");
    let mut msi_data = pkg.into_inner().expect("into_inner").into_inner();
    eprintln!("MSI after flush: {} bytes", msi_data.len());
    
    // Build cabinet using cab crate
    let cab_data = build_cabinet(file1_data, file2_data);
    eprintln!("Cabinet size: {} bytes", cab_data.len());
    
    // Add cabinet stream via cfb - use Cursor<Vec<u8>> so CFB can grow the buffer
    {
        let cursor = Cursor::new(msi_data);
        let mut comp = cfb::CompoundFile::open(cursor).expect("open CFB");
        let mut stream = comp.create_stream("ref.cab").expect("create cab stream");
        stream.write_all(&cab_data).expect("write cab");
        comp.flush().expect("flush CFB");
        let inner = comp.into_inner();
        msi_data = inner.into_inner();
    }
    
    eprintln!("MSI with cabinet: {} bytes", msi_data.len());
    
    let out_path = r"C:\Users\visse\OneDrive\Documentos\V.E.L.O.C.I.T.Y.-Installer-master\examples\sample-app\output\ref_msi_crate.msi";
    std::fs::write(out_path, &msi_data).expect("write MSI");
    eprintln!("Written to: {}", out_path);
    eprintln!("\nTest with: msiexec /i \"{}\" /qn /l*v log_ref.txt", out_path);
}

fn v(s: &str) -> Value { Value::Str(s.to_string()) }

/// Build cabinet using the cab crate
fn build_cabinet(file1: &[u8], file2: &[u8]) -> Vec<u8> {
    let mut cab_cursor = Cursor::new(Vec::new());
    {
        let mut builder = cab::CabinetBuilder::new();
        let folder = builder.add_folder(cab::CompressionType::None);
        {
            let mut f = folder.add_file("readme.txt");
            // FileBuilder doesn't have set_data - we write via FileWriter after build
        }
        {
            let mut f = folder.add_file("data.txt");
        }
        
        let mut cabinet = builder.build(&mut cab_cursor).expect("build cab");
        
        // Write file data via FileWriter
        if let Some(mut fw) = cabinet.next_file().expect("next_file 1") {
            fw.write_all(file1).expect("write file 1");
        }
        if let Some(mut fw) = cabinet.next_file().expect("next_file 2") {
            fw.write_all(file2).expect("write file 2");
        }
        
        cabinet.finish().expect("finish cab");
    }
    
    cab_cursor.into_inner()
}
