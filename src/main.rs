// extern crate ffmpeg_next as ffmpeg;
pub mod args;
pub mod convert_to_mp4;
pub mod decode_from_mp4;
pub mod qr_generation;
pub mod sort_lib;

use args::parse_args;
use decode_from_mp4::decode_from_mp4;
use qr_generation::create_qrcode_file;

use std::str::FromStr;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let default_framerate: String = String::from_str("30").unwrap();
    let default_output_file: String = String::from_str("output.mp4").unwrap();
    let mut decode: String = false.to_string();
    let parsed_arguments =
        parse_args(&args, &default_framerate, &default_output_file, &mut decode).unwrap();

    match parsed_arguments[3].unwrap().parse::<bool>().unwrap() {
        true => decode_from_mp4(&parsed_arguments),
        false => create_qrcode_file(&parsed_arguments),
    }
}
