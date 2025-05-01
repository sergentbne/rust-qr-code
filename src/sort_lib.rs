use std::cmp::Ordering;
use std::path::PathBuf;

pub fn sort_the_vector_right(a: &PathBuf, b: &PathBuf) -> Ordering {
    let mut a = String::from(
        a.file_name()
            .expect("The file name of a was missing")
            .to_str()
            .unwrap(),
    );
    let mut b = String::from(
        b.file_name()
            .expect("The file name of b was missing")
            .to_str()
            .unwrap(),
    );
    for i in [&mut a, &mut b] {
        *i = i.replace(".png", "");
    }

    let a_int: u32 = a.parse::<u32>().unwrap();
    let b_int: u32 = b.parse::<u32>().unwrap();

    return a_int.cmp(&b_int);
}
