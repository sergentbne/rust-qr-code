use std::str::FromStr;

pub fn parse_args<'a>(
    args: &'a Vec<String>,
    default_framerate: &'a String,
    default_video_output: &'a String,
    decode: &'a mut String,
) -> Result<[Option<&'a std::string::String>; 4], String> {
    let mut is_decode = false;
    let mut inputf: Option<&String> = None; //input file
    let mut outputf: Option<&String> = None; //output file
    let mut framerate: Option<&String> = Some(&default_framerate);

    for (index, value) in args.iter().enumerate() {
        match value.as_str() {
            "-i" | "--input" => inputf = Some(&args[index + 1]),
            "-o" | "--output" => outputf = Some(&args[index + 1]),
            "-f" | "--framerate" => framerate = Some(&args[index + 1]),
            "-d" | "--decode" => is_decode = true,

            &_ => {
                print!("")
            }
        }
    }
    *decode = is_decode.to_string();
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
    return Ok([inputf, outputf, framerate, Some(decode)]);
}
