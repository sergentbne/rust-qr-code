use crate::convert_to_mp4::convert_func;
use crate::decode_from_mp4::debug_data;
use image::{Luma, buffer};
use qrcode::QrCode;

use std::collections::VecDeque;
use std::fs::{File, create_dir, remove_dir_all};
use std::io::{Read, Write};
use xz::read::XzDecoder;
use xz::write::XzEncoder;

fn compress_file<'a>(
    path_to_file: &'a str,
    compressed: &'a mut VecDeque<u8>,
) -> Result<&'a mut VecDeque<u8>, String> {
    let mut data_from_file: Vec<u8> = vec![];

    let mut stuff = match File::open(path_to_file) {
        Ok(n) => n,
        Err(err) => return Err(format!("Error reading file: {}", err)),
    };

    let total = stuff.read_to_end(&mut data_from_file).unwrap();
    let mut encoder = XzEncoder::new(compressed, 6);
    match encoder.write_all(&data_from_file) {
        Ok(_) => (),
        Err(err) => return Err(format!("Error compressing file: {}", err)),
    }
    let compressed_final = encoder.finish().unwrap();

    println!("size before compression: {:}", total);
    #[cfg(debug_assertions)]
    {
        let data_clone = compressed_final.clone();
        let binding = Vec::from(data_clone);
        let data_temp = binding.as_slice();

        let mut shithead: Vec<u8> = Vec::new();
        let mut decoder_test: XzDecoder<&[u8]> = XzDecoder::new(data_temp);
        let stuff = decoder_test.read_to_end(&mut shithead).unwrap();
        assert!(stuff != 0);
        let mut temp_ninja_shit = std::fs::File::create("tmp2.xz").unwrap();
        temp_ninja_shit.write_all(&data_temp).unwrap();
        // println!("{:?}", shithead);
    }

    // assert!(decoder_test == data);
    Ok(compressed_final)
}

pub fn create_environement() -> () {
    match create_dir("/tmp/qrcode_files") {
        Ok(_) => {
            println!("Written directory in tmp for files")
        }
        Err(_err) => {
            clean_environnement();
            create_environement();
        }
    };
}

pub fn clean_environnement() {
    match remove_dir_all("/tmp/qrcode_files") {
        Ok(_) => {
            println!("Deleted directory in tmp for files")
        }
        Err(err) => panic!(
            "Failed to Remove directory, youll have to do it on your own: {}",
            err
        ),
    };
}
fn create_qr_code_from_data(data: &[u8], qr_number: u16) {
    let code: QrCode = QrCode::new(data).unwrap();

    let mut file_string: String = String::from("/tmp/qrcode_files/qrcode{}.png");
    let qr_number_string: String = qr_number.to_string();
    // Render the bits into an image.
    let image: image::ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>().build();

    file_string = file_string.replace("{}", &qr_number_string);
    image.save(file_string).unwrap();
    #[cfg(debug_assertions)]
    {
        debug_data(data, image.clone()).unwrap();
    }
    return;
}

fn get_data_from_file(data: &mut VecDeque<u8>) {
    let mut buffer: [u8; 2048] = [0; 2048];
    let mut qrcode_counter: u16 = 0;
    let mut interrupt = buffer.len();
    let mut total_size = 0;
    while data.len() != 0 {
        let mut tmp: Vec<u8>;
        for (i, elem) in &mut buffer.iter_mut().enumerate() {
            if let Some(front) = data.pop_front() {
                *elem = front;
            } else {
                interrupt = i;
                break;
            }
        }
        tmp = buffer.to_vec();
        tmp.truncate(interrupt);
        qrcode_counter += 1;
        total_size += tmp.len();
        println!("Total encoded: {}", total_size);
        create_qr_code_from_data(tmp.as_slice(), qrcode_counter);
    }
    println!("finished reading the file                                      ");
    println!("created {} image(s)", qrcode_counter);
}

pub fn create_qrcode_file(&parsed_arguments: &[Option<&String>; 4]) {
    create_environement();
    let mut compressions_deque = VecDeque::new();

    match compress_file(parsed_arguments[0].unwrap(), &mut compressions_deque) {
        Ok(n) => {
            println!("size after compression: {:}", &n.len());
            get_data_from_file(n);
        }
        Err(err) => panic!("compression error!, {}", err),
    };
    convert_func(parsed_arguments[1].unwrap(), parsed_arguments[2].unwrap());
    clean_environnement();
}
