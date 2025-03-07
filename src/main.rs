use image::Luma;
use qrcode::QrCode;
use std::fmt::Error;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;

fn create_qr_code_from_data(data: [u8; 1024], qr_number: u8) {
    let data_byte_string_litteral: &[u8] = &data;
    let code = QrCode::new(data_byte_string_litteral).unwrap();
    let mut file_string: String = String::from("/tmp/qrcode{}.png");
    let qr_number_string = qr_number.to_string();
    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();
    file_string = file_string.replace("{}", &qr_number_string);
    image.save(file_string).unwrap();
}

fn get_data_from_file(string_of_file: String) {
    let mut data_counter: u64 = 0;
    let mut qrcode_counter: u8 = 0;
    let mut file = match File::open(string_of_file) {
        Ok(file) => file,
        Err(err) => panic!("Error opening file: {}", err),
    };

    while data_counter < file.metadata().unwrap().len() {
        let offset: u64 = data_counter;
        match file.seek(SeekFrom::Start(offset)) {
            Ok(_) => println!("Seeked to offset {}", offset),
            Err(err) => panic!("Error seeking: {}", err),
        };

        let mut buffer = [0; 1024];
        match file.read(&mut buffer) {
            Ok(n) => println!("Read {} bytes", n),
            Err(err) => panic!("Error reading file: {}", err),
        };
        create_qr_code_from_data(buffer, qrcode_counter);
        data_counter += buffer.len() as u64;
        qrcode_counter += 1;
    }
}
fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("{}", args[1]);
    get_data_from_file(args[1].clone());
}
