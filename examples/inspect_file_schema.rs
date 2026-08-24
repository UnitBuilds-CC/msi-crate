/// Compare column bitfields between msi crate and velocity-msi
/// cargo run --example inspect_file_schema -p velocity-msi
fn main() {
    println!("=== msi crate File table column bitfields ===");
    {
        let cols: Vec<msi::Column> = vec![
            msi::Column::build("File_").primary_key().string(72),
            msi::Column::build("Component_").string(72),
            msi::Column::build("FileName").string(255).is_localizable(),
            msi::Column::build("FileSize").int32(),
            msi::Column::build("Attributes").nullable().int16(),
            msi::Column::build("Sequence").int16(),
        ];
        for (i, col) in cols.iter().enumerate() {
            println!("  Col {}: bitfield=0x{:04X} ({})", i + 1, col.bitfield(), col.bitfield());
        }
    }

    println!("\n=== velocity-msi File table column bitfields ===");
    {
        use velocity_msi::Column;
        let cols = vec![
            Column::build("File_").string(72).primary_key().build(),
            Column::build("Component_").string(72).build(),
            Column::build("FileName").string(255).localizable().build(),
            Column::build("FileSize").int32().build(),
            Column::build("Attributes").nullable().int16().build(),
            Column::build("Sequence").int16().build(),
        ];
        for (i, col) in cols.iter().enumerate() {
            println!("  Col {}: bitfield=0x{:04X} ({})", i + 1, col.bitfield(), col.bitfield());
        }
    }
}
