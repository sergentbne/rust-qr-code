use image::Luma;
use qrcode::QrCode;
use std::fs::File;
use std::io::Read;

fn create_qr_code_from_data(data: [u8; 2048], qr_number: u16) {
    let data_byte_string_litteral: &[u8] = &data;
    let code: QrCode = QrCode::new(data_byte_string_litteral).unwrap();
    // .render()
    // .quiet_zone(false)
    // .build();

    let mut file_string: String = String::from("/tmp/qrcode{}.png");
    let qr_number_string = qr_number.to_string();
    // Render the bits into an image.
    let image = code.render::<Luma<u8>>().build();
    file_string = file_string.replace("{}", &qr_number_string);
    image.save(file_string).unwrap();
}

fn get_data_from_file(string_of_file: String) {
    let mut data_counter: u64 = 0;
    let mut qrcode_counter: u16 = 0;
    let mut file = match File::open(string_of_file) {
        Ok(file) => file,
        Err(err) => panic!("Error opening file: {}", err),
    };
    let mut buffer = [0; 2048];
    let file_length = file.metadata().unwrap().len();
    while data_counter < file_length {
        match file.read(&mut buffer) {
            Ok(n) => println!("Read {} bytes out of {}", n, file_length),
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
