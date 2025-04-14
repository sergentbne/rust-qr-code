extern crate ffmpeg_next as ffmpeg;
use crate::qr_generation::{clean_environnement, create_environement};
use glob::glob;
use image::{GenericImage, GenericImageView, RgbImage};
use rqrr;
use std::fmt::write;
use std::fs;
use std::io::Write; // bring trait into scope
use std::path::PathBuf;

use ffmpeg::format::{Pixel, input};
use ffmpeg::media::Type;
use ffmpeg::software::scaling::{context::Context, flag::Flags};
use ffmpeg::util::frame::video::Video;
use std::env;
use std::fs::File;
use std::io::prelude::*;
use xz::write;

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
                    img.save(format!("/tmp/qrcode_files/frame{}.png", frame_index))
                        .unwrap();
                    println!("I went here!");
                    frame_index += 1;
                    // // Copy each line separately, accounting for padding
                    // for y in 0..frame_height {
                    //     let src_offset = y * frame_height * 3; // 3 bytes per pixel for RGB24
                    //     let dst_offset = y * frame_linesizes;

                    //     unsafe {
                    //         std::ptr::copy_nonoverlapping(
                    //             frame_data2.as_ptr().add(src_offset as usize),
                    //             frame_data.as_mut_ptr().add(dst_offset as usize),
                    //             (frame_width * 3) as usize,
                    //         );
                    //     }
                    // }

                    // // Verify frame data before saving
                    // if rgb_frame.format() != Pixel::RGB24 {
                    //     eprintln!(
                    //         "Warning: Output frame is not RGB24 (got {:?})",
                    //         rgb_frame.format()
                    //     );
                    // }
                    // let jpeg_path = Path::new("output.jpg");
                    // let mut jpeg_file = std::fs::File::create(jpeg_path).unwrap();
                    // let mut encoder = JpegEncoder::new(&mut jpeg_file);
                    // encoder
                    //     .encode(
                    //         &rgb_frame.data(0),
                    //         rgb_frame.width(),
                    //         rgb_frame.height(),
                    //         image::ExtendedColorType::Rgb8,
                    //     )
                    //     .unwrap();
                    // // save_ppm(&rgb_frame, frame_index).map_err(|e| ffmpeg::Error::Other { errno: 0 })?;

                    // frame_index += 1;
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
    let image_dir = "/tmp/qrcode_files";
    let _ = decode_vid(&parsed_arguments);
    let mut data: Vec<u8> = Vec::new();
    let mut data_from_img: Vec<u8>;

    let pattern = format!("{}/*.png", image_dir); // Change to your image extension

    let images: Vec<PathBuf> = glob(pattern.as_str())
        .expect("Failed to read glob pattern")
        .filter_map(Result::ok)
        .collect();
    println!("{:?}", images);

    for i in images {
        data_from_img = decode_img(i);
        println!("{:?}", data);
        data.append(&mut data_from_img);
    }

    let mut uncompressed = write::XzDecoder::new(data);
    let data = uncompressed.finish().unwrap();

    let mut file = fs::OpenOptions::new()
        .create(true)
        // .create(true) // To create a new file
        .write(true)
        // either use the ? operator or unwrap since it returns a Result
        .open(&parsed_arguments[1].unwrap())
        .unwrap();
    assert!(uncompressed.finish().unwrap().len() != 0);
    file.write_all(&data).unwrap();

    clean_environnement();
}

fn decode_img(img_path: PathBuf) -> Vec<u8> {
    let img = image::open(img_path).unwrap().to_luma8();
    // Prepare for detection
    let mut img = rqrr::PreparedImage::prepare(img);
    // Search for grids, without decoding
    let grids = img.detect_grids();
    assert_eq!(grids.len(), 1);
    let mut data = Vec::new();
    // Decode the grid
    let _ = grids[0].decode_to(&mut data).unwrap();
    assert!(data.len() != 0);
    println!("I went here!");
    return data;
}
