use std::{env::temp_dir, path::PathBuf};

pub fn get_tmp_folder() -> PathBuf {
    let qrcode_folder = "qrcode_files";
    let mut tmp_dir = temp_dir();
    tmp_dir.push(qrcode_folder);
    tmp_dir
}
