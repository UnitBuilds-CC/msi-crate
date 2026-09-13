/// Full hex dump of makecab cabinet
fn main() {
    let cab = std::fs::read("C:\\temp\\valid.cab").unwrap();
    println!("valid.cab: {} bytes", cab.len());
    for i in (0..cab.len()).step_by(16) {
        print!("{:04x}: ", i);
        for j in 0..16 {
            if i + j < cab.len() {
                print!("{:02x} ", cab[i+j]);
            } else {
                print!("   ");
            }
        }
        print!(" ");
        for j in 0..16 {
            if i + j < cab.len() {
                let b = cab[i+j];
                if (0x20..0x7F).contains(&b) {
                    print!("{}", b as char);
                } else {
                    print!(".");
                }
            }
        }
        println!();
    }
}
