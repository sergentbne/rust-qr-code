// extern crate ffmpeg_next as ffmpeg;
mod convert_to_mp4;

use convert_to_mp4::convert_func;

use ffmpeg_next::device::input;
use ffmpeg_next::option;
use ffmpeg_next::software::scaling::support::output;
use image::Luma;
use qrcode::QrCode;
use qrcode::render::string;

use std::collections::VecDeque;
use std::fs::{File, create_dir, remove_dir_all};
use std::io::{Read, Write};
use std::result;
use std::str::FromStr;
use xz::write::XzEncoder;

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
        Err(_err) => {
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
    println!("created {} image(s)", qrcode_counter);
}

fn parse_args<'a>(
    args: &'a Vec<String>,
    default_framerate: &'a String,
    default_video_output: &'a String,
) -> Result<[Option<&'a std::string::String>; 3], String> {
    let mut inputf: Option<&String> = None; //input file
    let mut outputf: Option<&String> = None; //output file
    let mut framerate: Option<&String> = Some(&default_framerate);

    for (index, value) in args.iter().enumerate() {
        match value.as_str() {
            "-i" | "--input" => inputf = Some(&args[index + 1]),
            "-o" | "--output" => outputf = Some(&args[index + 1]),
            "-f" | "--framerate" => framerate = Some(&args[index + 1]),

            &_ => {
                print!("")
            }
        }
    }
    if inputf.is_none() && outputf.is_none() && Some(&args[1]).is_some() {
        inputf = Some(&args[1]);
        outputf = Some(default_video_output);
    }
    if !inputf.is_some() {
        return Err(String::from_str("Input was not defined").unwrap());
    }
    if !outputf.is_some() {
        return Err(String::from_str("Output was not defined").unwrap());
    }
    return Ok([inputf, outputf, framerate]);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let default_framerate: String = String::from_str("30").unwrap();
    let default_output_file: String = String::from_str("output.mp4").unwrap();
    let parsed_arguments = parse_args(&args, &default_framerate, &default_output_file);
    let parsed_arg_bullshit = parsed_arguments
        .expect("parsed arguments should not be empty, pass some arguments")[0]
        .expect("...how did you get here?");
    create_environement();
    // println!("{}", args[1]);

    match compress_file(parsed_arg_bullshit) {
        Ok(n) => {
            println!("{}", parsed_arg_bullshit);
            get_data_from_file(n)
        }
        Err(err) => panic!("compression error!, {}", err),
    };
    create_mp4();
    clean_environnement();
}
