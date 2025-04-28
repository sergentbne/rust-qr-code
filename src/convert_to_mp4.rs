extern crate ffmpeg_next as ffmpeg;

use std::{
    path::PathBuf,
    thread,
    time::{self, Duration},
};

use ffmpeg::{Dictionary, Packet, Rational, codec, encoder, format, frame, log};
use glob::glob;
use image::imageops::FilterType;

const DEFAULT_X264_OPTS: &str = "";
struct Transcoder {
    encoder: encoder::Video,
    frame_count: usize,
    framerate: u8,
    images: Vec<PathBuf>,
    scaler: ffmpeg::software::scaling::Context,
    data_encoded_numb: u32,
    octx: format::context::Output,
    frame_count_for_packets: u32,
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
        framerate: u8,
        mut octx: format::context::Output,
        x264_opts: Dictionary,
    ) -> Result<Self, ffmpeg::Error> {
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
        encoder.set_frame_rate(Some(Rational::new(framerate as i32, 1))); // 30 FPS
        encoder.set_bit_rate(8_000_000);
        encoder.set_max_bit_rate(10_000_000);
        encoder.set_gop(12);
        encoder.set_quality(23);

        let mut opened_encoder = encoder
            .open_with(x264_opts)
            .expect("error opening x264 with supplied settings");
        opened_encoder.set_frame_rate(Some(Rational::new(framerate as i32, 1)));
        let scaler = ffmpeg::software::scaling::Context::get(
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
        ost.set_avg_frame_rate(Rational::new(framerate as i32, 1));

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
            encoder: opened_encoder,
            frame_count: 0,
            framerate: framerate,
            images: images,
            scaler: scaler,
            data_encoded_numb: 0,
            octx: octx,
            frame_count_for_packets: 0,
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
        // let frame_width = frame.width() as usize;
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

        for image_frame in self.images.clone() {
            let mut yuv_frame: frame::Video =
                frame::Video::new(format::Pixel::YUV420P, width, height);

            let img = image::ImageReader::open(image_frame)
                .unwrap()
                .decode()
                .unwrap()
                .resize_exact(width, height, FilterType::Lanczos3)
                .to_rgb8();

            Transcoder::copy_image_to_frame(&mut rgb_frame, &img, width as usize, height as usize)
                .unwrap();

            self.scaler.run(&rgb_frame, &mut yuv_frame).unwrap();

            // Properly set frame data for YUV420P

            yuv_frame.set_pts(Some(self.frame_count as i64));
            self.send_frame_to_encoder(&yuv_frame);
            self.frame_count += 1 * (15360 / self.framerate as usize);
        }
    }

    fn send_frame_to_encoder(&mut self, frame: &frame::Video) {
        loop {
            match self.encoder.send_frame(frame) {
                Ok(_) => {
                    println!("It works! {} images done!", self.data_encoded_numb);
                    self.data_encoded_numb += 1;
                    break;
                }
                Err(err) => {
                    println!("Gosh dang it!, {}", err);
                    self.recieve_packets_and_flush();
                }
            }
        }
    }

    fn send_eof_to_encoder(&mut self) {
        self.encoder.send_eof().unwrap();
    }

    fn recieve_packets_and_flush(&mut self) {
        let mut packet: Packet = Packet::empty();

        loop {
            match self.encoder.receive_packet(&mut packet) {
                Ok(()) => {
                    packet.set_stream(0);

                    match packet.write_interleaved(&mut self.octx) {
                        Ok(_) => println!("Packet written successfully."),
                        Err(e) => eprintln!("Error writing packet: {}", e),
                    }

                    packet = Packet::empty(); // Clear the packet for next use
                }
                Err(ffmpeg::Error::Other {
                    errno: ffmpeg::util::error::EAGAIN,
                }) => {
                    println!("encoding is done");
                    break;
                }
                Err(ffmpeg::Error::Eof) => {
                    break;
                }
                Err(e) => {
                    eprintln!("Error receiving packet: {}", e);
                    break;
                }
            }
            self.frame_count_for_packets += 1;
        }
    }
}

pub fn convert_func(output_file: &String, framerate: &String) {
    let image_dir = "/tmp/qrcode_files";
    let framerate_int: u8 = framerate.parse::<u8>().unwrap();
    let pattern = format!("{}/*.png", image_dir); // Change to your image extension
    println!("pattern: {}", pattern);
    let x264_opts = parse_opts(DEFAULT_X264_OPTS.to_string()).unwrap();

    eprintln!("x264 options: {:?}", x264_opts);

    ffmpeg::init().unwrap();
    log::set_level(log::Level::Debug);

    let mut octx = format::output(output_file).unwrap();

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
        framerate_int,
        octx,
        x264_opts,
    )
    .unwrap();
    // println!(
    //     "Output stream time base: {:?}",
    //     octx.stream(0).unwrap().time_base()
    // );
    transcoder.receive_and_process_decoded_frames();

    // assert_eq!(
    //     octx.stream(0).unwrap().avg_frame_rate(),
    //     transcoder.encoder.frame_rate()
    // );
    println!("Encoder frame rate: {:?}", transcoder.encoder.frame_rate());
    // Process frames
    // transcoder.receive_and_process_decoded_frames();
    transcoder.send_eof_to_encoder();
    // FLUSH THE ENCODER PROPERLY
    let mut packet: Packet = Packet::empty();
    loop {
        match transcoder.encoder.receive_packet(&mut packet) {
            Ok(()) => {
                packet.set_stream(0);

                match packet.write_interleaved(&mut transcoder.octx) {
                    Ok(_) => println!("Packet written successfully."),
                    Err(e) => eprintln!("Error writing packet: {}", e),
                }

                packet = Packet::empty(); // Clear the packet for next use
            }
            Err(ffmpeg::Error::Other {
                errno: ffmpeg::util::error::EAGAIN,
            }) => {
                break;
            }
            Err(ffmpeg::Error::Eof) => {
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
        transcoder.octx.stream(0).unwrap().avg_frame_rate(),
        transcoder.encoder.frame_rate()
    );
    //     transcoder.send_eof_to_encoder();
    // println!("done that");
    // Now write trailer
    if let Err(e) = transcoder.octx.write_trailer() {
        eprintln!("Failed to write trailer: {}", e);
    }
    println!(
        "stuff: {:?}",
        transcoder.octx.stream(0).unwrap().avg_frame_rate()
    );
}
