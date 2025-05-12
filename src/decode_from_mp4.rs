extern crate ffmpeg_next as ffmpeg;

use crate::get_path_of_temp::get_tmp_folder;
use crate::qr_generation::{clean_environnement, create_environement};
use glob::glob;
use image::{Luma, RgbImage};
use rqrr;
use std::fs;
use std::io::Write;
// bring trait into scope
use crate::sort_lib::sort_the_vector_right;
use std::path::PathBuf;

use ffmpeg::format::{Pixel, input};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::io::prelude::*;
use xz::read::XzDecoder;

fn decode_vid(parsed_arguments: &[Option<&String>; 4]) -> Result<(), ffmpeg::Error> {
    create_environement();
    ffmpeg::init().unwrap();
    let inputf = parsed_arguments[0].unwrap();

    if let Ok(mut ictx) = input(&inputf) {
        let input = ictx
            .streams()
            .best(Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;
        let video_stream_index = input.index();

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;
        let mut decoder = context_decoder.decoder().video()?;

        let mut scaler = Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            Flags::BILINEAR,
        )?;

        let mut frame_index = 0;

        let mut process_decoded_frames =
            |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
                let mut decoded = Video::empty();
                while decoder.receive_frame(&mut decoded).is_ok() {
                    let mut rgb_frame = Video::empty();
                    scaler.run(&decoded, &mut rgb_frame)?;
                    let frame_format = rgb_frame.format();
                    // let frame_width = frame.width() as usize;
                    let frame_height = rgb_frame.height();
                    let frame_width = rgb_frame.width();
                    let frame_linesizes = rgb_frame.stride(0);

                    // For simplicity, assuming RGB24 format - adjust for your needs
                    if frame_format != ffmpeg::format::Pixel::RGB24 {
                        return Err(ffmpeg::Error::InvalidData);
                    }

                    // Get mutable access to frame data
                    let frame_data = rgb_frame.data(0);
                    let mut img: RgbImage =
                        image::ImageBuffer::new(frame_width as u32, frame_height as u32);
                    for y in 0..frame_height {
                        let row_start = (y as usize) * frame_linesizes;
                        let row_end = row_start + (frame_width as usize) * 3;
                        let row_data = &frame_data[row_start..row_end];
                        for x in 0..frame_width {
                            let pixel = &row_data[(x as usize) * 3..][..3];
                            img.put_pixel(x, y, image::Rgb([pixel[0], pixel[1], pixel[2]]));
                        }
                    }
                    let frame_index_str = format!("{}.png", frame_index);
                    let mut tmp_img_dir = get_tmp_folder();
                    tmp_img_dir.push(frame_index_str);
                    img.save(tmp_img_dir).unwrap();
                    println!("{}.png decoded!", frame_index);
                    frame_index += 1;
                }
                Ok(())
            };
        for (stream, packet) in ictx.packets() {
            if stream.index() == video_stream_index {
                decoder.send_packet(&packet)?;
                process_decoded_frames(&mut decoder)?;
            }
        }

        decoder.send_eof()?;
        process_decoded_frames(&mut decoder)?;
    }
    Ok(())
}

pub fn decode_from_mp4(parsed_arguments: &[Option<&String>; 4]) {
    let image_dir = get_tmp_folder();
    let _ = decode_vid(parsed_arguments).unwrap();
    let mut data: Vec<u8> = Vec::new();
    let mut data_from_img: Vec<u8>;

    let pattern = format!("{}/*.png", image_dir.display()); // Change to your image extension

    let mut images: Vec<PathBuf> = glob(pattern.as_str())
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect();
    images.sort_by(|a, b| sort_the_vector_right(a, b));

    for i in images {
        data_from_img = decode_img(i);
        for y in data_from_img {
            data.push(y);
        }
        println!("{:?} bytes", &data.len());
    }
    // data = data[..=int_input as usize].to_vec();
    let mut buffer_vec: Vec<u8> = Vec::new();

    let mut uncompressed = XzDecoder::new(data.as_slice());
    let mut data_from_file =
        || -> Result<usize, std::io::Error> { Ok(uncompressed.read_to_end(&mut buffer_vec)?) };
    // = uncompressed.read_to_end(&mut buffer);
    if let Err(err) = data_from_file() {
        panic!("Error in Smth idk: {}", err)
    };

    let mut file = fs::OpenOptions::new()
        .create(true)
        // .create(true) // To create a new file
        .write(true)
        // either use the ? operator or unwrap since it returns a Result
        .open(&parsed_arguments[1].unwrap())
        .unwrap();
    // assert!(uncompressed.finish().unwrap().len() != 0);
    if !buffer_vec.is_empty() {
        file.write_all(&buffer_vec).unwrap();
    }

    clean_environnement();
}

fn decode_img(img_path: PathBuf) -> Vec<u8> {
    let filename = img_path.file_name().expect("lol what?").to_str().unwrap();
    let img = image::open(&img_path).unwrap().to_luma8();
    // Prepare for detection
    let mut img = rqrr::PreparedImage::prepare(img);
    // Search for grids, without &decoding
    let grids = img.detect_grids();
    if grids.len() != 1 {
        for i in &grids {
            println!("grid bounds {:?}", i.bounds)
        }
        println!("Detected no or multiple qrcodes, exiting...");
    }
    let mut data = Vec::new();
    // Decode the grid
    let _ = grids[0].decode_to(&mut data).unwrap();
    assert!(data.len() != 0);
    println!("{} is done!", filename);
    return data;
}

fn decode_img_with_data(img_data: image::ImageBuffer<Luma<u8>, Vec<u8>>) -> Vec<u8> {
    // Prepare for detection

    let mut img = rqrr::PreparedImage::prepare(img_data);
    // Search for grids, without decoding
    let grids = img.detect_grids();

    // if grids.len() != 1 {
    //     for i in &grids {
    //         println!("grid bounds {:?}", i.bounds)
    //     }
    //     println!("Detected no or multiple qrcodes, exiting...");
    // }

    let mut data = Vec::new();
    // Decode the grid
    let _ = grids[0].decode_to(&mut data).unwrap();
    assert!(data.len() != 0);
    return data;
}

pub fn debug_data(
    encoded_data: &[u8],
    image_data: image::ImageBuffer<Luma<u8>, Vec<u8>>,
) -> Result<(), String> {
    let is_data_same = encoded_data != decode_img_with_data(image_data).as_slice();
    if is_data_same {
        println!("Assertion failed, program will exist now: {}", is_data_same);
        return Err(String::from("AAAAAAAH"));
    }

    Ok(())
}
