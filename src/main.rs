extern crate ffmpeg_next as ffmpeg;
use image::{GenericImageView, Luma};
use qrcode::QrCode;
// use std::fmt::Error;
use gif::Encoder;
use rust_qr_code::convert_to_av1::convert_func;

use std::collections::VecDeque;
use std::fs::{File, create_dir, read_dir, remove_dir_all};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use xz::write::XzEncoder;

use std::collections::HashMap;
use std::env;
use std::time::Instant;

fn is_image_file(entry: &PathBuf) -> bool {
    if let Some(ext) = entry.extension().and_then(|s| s.to_str()) {
        let ext = ext.to_lowercase();
        return ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "bmp";
    }
    false
}

fn create_mp4() {
    convert_func();
}

fn compress_file(path_to_file: &str) -> Result<VecDeque<u8>, String> {
    let mut data: Vec<u8> = vec![];

    let mut stuff = match File::open(path_to_file) {
        Ok(n) => n,
        Err(err) => return Err(format!("Error reading file: {}", err)),
    };

    stuff.read_to_end(&mut data).unwrap();
    let mut compressed = VecDeque::new();
    let mut encoder = XzEncoder::new(&mut compressed, 9);
    match encoder.write_all(&data) {
        Ok(_) => (),
        Err(err) => return Err(format!("Error compressing file: {}", err)),
    }
    match encoder.finish() {
        Ok(_) => (),
        Err(err) => return Err(format!("Error finishing compression: {}", err)),
    };
    println!(
        "size before compression: {:}",
        stuff.metadata().unwrap().len()
    );
    println!("size after compression: {:}", compressed.len());
    Ok(compressed)
}

fn create_environement() -> () {
    match create_dir("/tmp/qrcode_files") {
        Ok(_) => {
            println!("Written directory in tmp for files")
        }
        Err(err) => {
            clean_environnement();
            create_environement();
        }
    };
}

fn clean_environnement() {
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
fn create_qr_code_from_data(data: [u8; 2048], qr_number: u16) {
    let data_byte_string_litteral: &[u8] = &data;
    let code: QrCode = QrCode::new(data_byte_string_litteral).unwrap();

    let mut file_string: String = String::from("/tmp/qrcode_files/qrcode{}.png");
    let qr_number_string: String = qr_number.to_string();
    // Render the bits into an image.
    let image: image::ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>().build();
    file_string = file_string.replace("{}", &qr_number_string);
    image.save(file_string).unwrap();
    return;
}

fn get_data_from_file(mut data: VecDeque<u8>) {
    let mut buffer: [u8; 2048] = [0; 2048];
    let mut qrcode_counter: u16 = 0;
    while data.len() != 0 {
        for elem in &mut buffer {
            if let Some(front) = data.pop_back() {
                *elem = front;
            } else {
                break;
            }
        }
        qrcode_counter += 1;
        create_qr_code_from_data(buffer, qrcode_counter);
    }
    println!("finished reading the file                                      ");
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    create_environement();
    // println!("{}", args[1]);

    match compress_file(&args[1]) {
        Ok(n) => get_data_from_file(n),
        Err(err) => panic!("something went wrong here, {}", err),
    };
    create_mp4();
    clean_environnement();
}
