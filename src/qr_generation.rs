use crate::convert_to_mp4::convert_func;
use crate::decode_from_mp4::debug_data;
use crate::get_path_of_temp::get_tmp_folder;
use image::Luma;
use qrcode::QrCode;

use std::collections::VecDeque;
use std::fs::{File, create_dir, remove_dir_all};
use std::io::{Read, Write};
use std::thread;
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

        let mut final_data: Vec<u8> = Vec::new();
        let mut decoder_test: XzDecoder<&[u8]> = XzDecoder::new(data_temp);
        let qty_of_bytes_in_data = decoder_test.read_to_end(&mut final_data).unwrap();
        assert!(qty_of_bytes_in_data != 0);

        // println!("{:?}", shithead);
    }

    // assert!(decoder_test == data);
    Ok(compressed_final)
}

pub fn create_environement() -> () {
    match create_dir(get_tmp_folder()) {
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
    match remove_dir_all(get_tmp_folder()) {
        Ok(_) => {
            println!("Deleted directory in tmp for files")
        }
        Err(err) => panic!(
            "Failed to Remove directory, youll have to do it on your own: {}",
            err
        ),
    };
}
fn create_qr_code_from_data(data: &[u8], qr_number: &u32) {
    let code: QrCode = QrCode::new(data).unwrap();

    let mut file_string: String = String::from("[]/{}.png");
    let qr_number_string: String = qr_number.to_string();

    // Render the bits into an image.
    let image: image::ImageBuffer<Luma<u8>, Vec<u8>> = code.render::<Luma<u8>>().build();

    file_string = file_string
        .replace("{}", &qr_number_string)
        .replace("[]", get_tmp_folder().to_str().expect("lolla"));

    image.save(file_string).unwrap();
    #[cfg(debug_assertions)]
    {
        debug_data(data, image.clone()).unwrap();
    }
    return;
}

fn get_data_from_file(data: &mut VecDeque<u8>) -> u32 {
    let mut buffer: [u8; 2048] = [0; 2048];

    let mut qrcode_counter: u32 = 0;
    let mut interrupt = buffer.len();
    let mut total_size = 0;
    let mut threads = Vec::new();
    let total_data_of_file = data.len().clone() as f32;
    let max_threads: u8 = 10;
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

        print!(
            "\rTotal encoded: {}/{} [{}%]",
            total_size,
            total_data_of_file,
            (total_size as f32 / total_data_of_file * 100 as f32).floor()
        );
        std::io::stdout().flush().unwrap();

        while threads.len() >= max_threads as usize {
            let handle: thread::JoinHandle<()> = threads.remove(0);
            handle.join().unwrap();
        }

        threads.push(thread::spawn(move || {
            create_qr_code_from_data(tmp.as_slice(), &qrcode_counter);
        }));
        while threads.len() != 0 {
            let handle: thread::JoinHandle<()> = threads.remove(0);
            handle.join().unwrap();
        }
    }

    println!("\nfinished reading the file                                      ");
    println!("created {} image(s)", qrcode_counter);
    qrcode_counter
}

pub fn create_qrcode_file(&parsed_arguments: &[Option<&String>; 4]) {
    create_environement();
    let mut compressions_deque = VecDeque::new();
    let nb_of_images: u32;

    match compress_file(parsed_arguments[0].unwrap(), &mut compressions_deque) {
        Ok(n) => {
            println!("size after compression: {:}", &n.len());
            nb_of_images = get_data_from_file(n);
        }
        Err(err) => panic!("compression error!, {}", err),
    };
    convert_func(
        parsed_arguments[1].unwrap(),
        parsed_arguments[2].unwrap(),
        nb_of_images,
    );
    clean_environnement();
}
