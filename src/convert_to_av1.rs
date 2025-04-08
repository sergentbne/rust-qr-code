extern crate ffmpeg_next as ffmpeg;

use std::env;
use std::time::Instant;
use std::{collections::HashMap, path::PathBuf};

use ffmpeg::{
    Dictionary, Packet, Rational, codec, decoder, encoder, format, frame, log, media, picture,
};
use ffmpeg_next::ffi::av_hwdevice_iterate_types;
use glob::glob;
use image::imageops::FilterType;
use image::{EncodableLayout, GenericImage, RgbImage};
use qrcode::render::string;

// const DEFAULT_X264_OPTS: &str = "preset=medium";
const DEFAULT_X264_OPTS: &str = "fps=30";

struct Transcoder {
    ost_index: usize,
    encoder: encoder::Video,
    logging_enabled: bool,
    frame_count: usize,
    last_log_frame_count: usize,
    starting_time: Instant,
    last_log_time: Instant,
    images: Vec<PathBuf>,
    scaler: ffmpeg::software::scaling::Context,
}

fn parse_opts<'a>(s: String) -> Option<Dictionary<'a>> {
    let mut dict = Dictionary::new();
    for keyval in s.split_terminator(',') {
        let tokens: Vec<&str> = keyval.split('=').collect();
        match tokens[..] {
            [key, val] => dict.set(key, val),
            _ => return None,
        }
    }
    Some(dict)
}

impl Transcoder {
    fn new(
        width: u32,
        height: u32,
        pattern: &str,
        octx: &mut format::context::Output,
        ost_index: usize,
        x264_opts: Dictionary,
        enable_logging: bool,
    ) -> Result<Self, ffmpeg::Error> {
        // let global_header = octx.format().flags().contains(format::Flags::GLOBAL_HEADER);

        let codec = encoder::find(codec::Id::H264);
        let mut ost = octx.add_stream(codec)?;

        let mut encoder =
            codec::context::Context::new_with_codec(codec.ok_or(ffmpeg::Error::InvalidData)?)
                .encoder()
                .video()?;

        encoder.set_height(height);
        encoder.set_width(width);
        encoder.set_format(format::Pixel::YUV420P);
        encoder.set_time_base(Rational::new(1, 15360));
        encoder.set_frame_rate(Some(Rational::new(30, 1))); // 30 FPS
        encoder.set_bit_rate(8_000_000);
        encoder.set_max_bit_rate(10_000_000);
        encoder.set_gop(12);
        encoder.set_quality(23);

        let mut opened_encoder = encoder
            .open_with(x264_opts)
            .expect("error opening x264 with supplied settings");
        opened_encoder.set_frame_rate(Some(Rational::new(30, 1)));
        let mut scaler = ffmpeg::software::scaling::Context::get(
            ffmpeg::format::Pixel::RGB24,
            width,
            height,
            ffmpeg::format::Pixel::YUV420P,
            width,
            height,
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .unwrap();

        ost.set_parameters(&opened_encoder);
        ost.set_time_base(Rational::new(1, 15360));
        ost.set_avg_frame_rate(Rational::new(30, 1));

        // ost.set_rate(Rational::new(30, 1));
        println!(
            "Output stream (from ost) time base in transcoder constructor: {:?}",
            ost.time_base()
        );

        assert_eq!(ost.avg_frame_rate(), opened_encoder.frame_rate());
        assert_eq!(
            octx.stream(0).unwrap().time_base(),
            opened_encoder.time_base()
        );
        match octx.write_header() {
            Ok(_) => println!("Header written successfully."),
            Err(e) => eprintln!("Failed to write header: {}", e),
        }
        println!(
            "Output stream (from octx) time base in transcoder constructor: {:?}",
            octx.stream(0).unwrap().time_base()
        );
        println!(
            "Output stream (from octx) time base in transcoder constructor: {:?}",
            &octx.stream(0).unwrap().time_base()
        );
        println!(
            "Encoder frame rate in transcoder constructor {:?}",
            opened_encoder.frame_rate()
        );

        println!("All output context streams {:?}", &octx.streams().count());
        let images: Vec<PathBuf> = glob(pattern)
            .expect("Failed to read glob pattern")
            .filter_map(Result::ok)
            .collect();

        Ok(Self {
            ost_index,
            encoder: opened_encoder,
            logging_enabled: enable_logging,
            frame_count: 0,
            last_log_frame_count: 0,
            starting_time: Instant::now(),
            last_log_time: Instant::now(),
            images: images,
            scaler: scaler,
        })
    }
    fn copy_image_to_frame(
        frame: &mut ffmpeg::frame::Video,
        image_data: &[u8],
        image_width: usize,
        image_height: usize,
    ) -> Result<(), ffmpeg::Error> {
        // Verify frame dimensions match expectations
        if frame.width() as usize != image_width || frame.height() as usize != image_height {
            return Err(ffmpeg::Error::InvalidData);
        }

        // Get frame parameters
        let frame_format = frame.format();
        let frame_width = frame.width() as usize;
        let frame_height = frame.height() as usize;
        let frame_linesizes = frame.stride(0);

        // For simplicity, assuming RGB24 format - adjust for your needs
        if frame_format != ffmpeg::format::Pixel::RGB24 {
            return Err(ffmpeg::Error::InvalidData);
        }

        // Get mutable access to frame data
        let frame_data = frame.data_mut(0);

        // Copy each line separately, accounting for padding
        for y in 0..frame_height {
            let src_offset = y * image_width * 3; // 3 bytes per pixel for RGB24
            let dst_offset = y * frame_linesizes as usize;

            unsafe {
                std::ptr::copy_nonoverlapping(
                    image_data.as_ptr().add(src_offset),
                    frame_data.as_mut_ptr().add(dst_offset),
                    image_width * 3,
                );
            }
        }

        Ok(())
    }
    fn receive_and_process_decoded_frames(&mut self) {
        // Set frame properties
        let (width, height) = if !self.images.is_empty() {
            let img = image::open(&self.images[0]).unwrap().to_rgb8();
            (img.width(), img.height())
        } else {
            return;
        };
        let mut rgb_frame: frame::Video = frame::Video::new(format::Pixel::RGB24, width, height);

        // unsafe {
        //     rgb_frame.alloc(ffmpeg::format::Pixel::RGB24, width, height);
        // }
        // unsafe {
        //     yuv_frame.alloc(ffmpeg::format::Pixel::YUV420P, width, height);
        // }
        for image_frame in self.images.clone() {
            let mut yuv_frame: frame::Video =
                frame::Video::new(format::Pixel::YUV420P, width, height);

            // let img =
            //     image::open(image_frame).unwrap().to_rgb8();
            let img = image::ImageReader::open(image_frame)
                .unwrap()
                .decode()
                .unwrap()
                .resize_exact(width, height, FilterType::Lanczos3)
                .to_rgb8();
            println!("{}, {}", img.as_bytes().len(), width * height * 3);

            Transcoder::copy_image_to_frame(&mut rgb_frame, &img, width as usize, height as usize)
                .unwrap();
            // let img_data = img.as_bytes();
            // let frame_data = rgb_frame.data_mut(0);
            // for (i, pixel) in img.pixels().enumerate() {
            //     let offset = i * 3;
            //     frame_data[offset] = pixel[0]; // R
            //     frame_data[offset + 1] = pixel[1]; // G
            //     frame_data[offset + 2] = pixel[2]; // B
            // }
            // for i in (0..width) {
            //     for y in (0..height) {}
            // }
            self.scaler.run(&rgb_frame, &mut yuv_frame).unwrap();

            // Properly set frame data for YUV420P

            yuv_frame.set_pts(Some(self.frame_count as i64));
            self.send_frame_to_encoder(&yuv_frame);
            self.frame_count += 1 * 512;
        }
    }

    fn send_frame_to_encoder(&mut self, frame: &frame::Video) {
        self.encoder.send_frame(frame).unwrap();
    }

    fn send_eof_to_encoder(&mut self) {
        self.encoder.send_eof().unwrap();
    }

    fn log_progress(&mut self, timestamp: f64) {
        if !self.logging_enabled
            || (self.frame_count - self.last_log_frame_count < 100
                && self.last_log_time.elapsed().as_secs_f64() < 1.0)
        {
            return;
        }
        eprintln!(
            "time elpased: \t{:8.2}\tframe count: {:8}\ttimestamp: {:8.2}",
            self.starting_time.elapsed().as_secs_f64(),
            self.frame_count,
            timestamp
        );
        self.last_log_frame_count = self.frame_count;
        self.last_log_time = Instant::now();
    }
}

// fn create_yuv_image(rgb_img: &RgbImage) -> (Vec<u8>, (u32, u32)) {
//     let width = rgb_img.width();
//     let height = rgb_img.height();
//     let mut y_plane = Vec::with_capacity((width * height) as usize);
//     let mut u_plane = Vec::with_capacity((width * height / 4) as usize);
//     let mut v_plane = Vec::with_capacity((width * height / 4) as usize);

//     // Simple RGB to YUV420 conversion (with subsampling)
//     for y in 0..height {
//         for x in 0..width {
//             let pixel = rgb_img.get_pixel(x, y);
//             let r = pixel[0] as f32;
//             let g = pixel[1] as f32;
//             let b = pixel[2] as f32;

//             // Y component
//             y_plane.push((0.299 * r + 0.587 * g + 0.114 * b).round() as u8);

//             // Subsample U and V (simple 2x2 average)
//             if x % 2 == 0 && y % 2 == 0 {
//                 let u = (-0.169 * r - 0.331 * g + 0.5 * b + 128.0)
//                     .round()
//                     .clamp(0.0, 255.0) as u8;
//                 let v = (0.5 * r - 0.419 * g - 0.081 * b + 128.0)
//                     .round()
//                     .clamp(0.0, 255.0) as u8;
//                 u_plane.push(u);
//                 v_plane.push(v);
//             }
//         }
//     }

//     let mut yuv_data = y_plane;
//     yuv_data.extend(u_plane);
//     yuv_data.extend(v_plane);
//     (yuv_data, (width, height))
// }

pub fn convert_func() {
    let image_dir = "/tmp/qrcode_files";
    let pattern = format!("{}/*.png", image_dir); // Change to your image extension
    println!("pattern: {}", pattern);
    let output_file = "output.mp4".to_string();
    let x264_opts = parse_opts(DEFAULT_X264_OPTS.to_string()).unwrap();

    eprintln!("x264 options: {:?}", x264_opts);

    ffmpeg::init().unwrap();
    log::set_level(log::Level::Debug);

    let mut octx = format::output(&output_file).unwrap();

    let mut ost_index: usize = 0;

    // Set up for stream copy for non-video stream.

    // We need to set codec_tag to 0 lest we run into incompatible codec tag
    // issues when muxing into a different container format. Unfortunately
    // there's no high level API to do this (yet).

    format::context::output::dump(&octx, 0, Some(&output_file));
    // octx.write_header().unwrap();
    let img_temp = image::open("/tmp/qrcode_files/qrcode1.png").unwrap();

    let mut transcoder = Transcoder::new(
        img_temp.width(),
        img_temp.height(),
        &pattern,
        &mut octx,
        ost_index,
        x264_opts,
        false,
    )
    .unwrap();
    println!(
        "Output stream time base: {:?}",
        octx.stream(0).unwrap().time_base()
    );
    transcoder.receive_and_process_decoded_frames();

    // Flush encoders and decoders.let img_temp = image::open("/tmp/qrcode_files/qrcode1.png").unwrap();
    // let mut transcoder = match Transcoder::new(
    //     img_temp.width(),
    //     img_temp.height(),
    //     &pattern,
    //     &mut octx,
    //     ost_index,
    //     x264_opts,
    //     false,
    // ) {
    //     Ok(t) => t,
    //     Err(e) => {
    //         eprintln!("Failed to create transcoder: {}", e);
    //         return;
    //     }
    // };
    println!(
        "Stream frame rate: {:?}",
        octx.stream(0).unwrap().avg_frame_rate()
    );

    assert_eq!(
        octx.stream(0).unwrap().avg_frame_rate(),
        transcoder.encoder.frame_rate()
    );
    transcoder.frame_count = 0;
    println!("Encoder frame rate: {:?}", transcoder.encoder.frame_rate());
    // Process frames
    // transcoder.receive_and_process_decoded_frames();
    transcoder.send_eof_to_encoder();
    // FLUSH THE ENCODER PROPERLY
    let mut packet = Packet::empty();
    loop {
        match transcoder.encoder.receive_packet(&mut packet) {
            Ok(()) => {
                packet.set_stream(0);
                // packet.set_pts(Some(transcoder.frame_count as i64));
                // packet.set_dts(Some(transcoder.frame_count as i64));
                // packet.set_time_base(Rational::new(1, 15360));
                // packet.rescale_ts(
                //     Rational::new(1, 15360), // Input timebase (should match encoder setting)
                //     octx.stream(0).unwrap().time_base(), // Output timebase
                // );
                match packet.write_interleaved(&mut octx) {
                    Ok(_) => println!("Packet written successfully."),
                    Err(e) => eprintln!("Error writing packet: {}", e),
                }

                packet = Packet::empty(); // Clear the packet for next use
            }
            Err(ffmpeg::Error::Other {
                errno: ffmpeg::util::error::EAGAIN,
            }) => {
                // Encoder needs more input, but we've already sent EOF, so we're done
                break;
            }
            Err(ffmpeg::Error::Eof) => {
                // Encoder is fully flushed
                break;
            }
            Err(e) => {
                eprintln!("Error receiving packet: {}", e);
                break;
            }
        }
        transcoder.frame_count += 1;
    }
    assert_eq!(
        octx.stream(0).unwrap().avg_frame_rate(),
        transcoder.encoder.frame_rate()
    );
    //     transcoder.send_eof_to_encoder();
    // println!("done that");
    // Now write trailer
    println!("done that");
    if let Err(e) = octx.write_trailer() {
        eprintln!("Failed to write trailer: {}", e);
    }
    println!("stuff: {:?}", octx.stream(0).unwrap().avg_frame_rate());
}
