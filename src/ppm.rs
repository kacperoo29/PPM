use std::{
    error::Error,
    fs::File,
    io::{BufReader, Read},
};

use crate::image::{BitmapData, Image};

pub struct PPM {
    width: usize,
    height: usize,
    max_value: usize,
    ver: PPMVer,
    buffer: BitmapData,
}

#[derive(PartialEq, Eq)]
pub enum PPMVer {
    P3,
    P6,
    None,
}

impl PPM {
    pub fn from_file(file_path: &str) -> Self {
        let mut ppm = PPM {
            width: 0,
            height: 0,
            max_value: 0,
            ver: PPMVer::None,
            buffer: BitmapData::None,
        };

        ppm.populate_from_file(file_path)
            .expect("Couldn't parse ppm file.");

        return ppm;
    }

    pub fn populate_from_file(&mut self, file_path: &str) -> Result<(), Box<dyn Error>> {
        let mut buffer = Vec::new();

        {
            let file = File::open(file_path)?;
            let mut reader = BufReader::new(file);
            reader.read_to_end(&mut buffer)?;
        }

        self.populate_from_buffer(&mut buffer)?;

        return Ok(());
    }

    pub fn from_buffer(buffer: &mut Vec<u8>) -> Self {
        let mut ppm = PPM {
            width: 0,
            height: 0,
            max_value: 0,
            ver: PPMVer::None,
            buffer: BitmapData::None,
        };

        ppm.populate_from_buffer(buffer)
            .expect("Couldn't parse ppm file.");

        return ppm;
    }

    fn populate_from_buffer(&mut self, buffer: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut is_commented = false;
        let mut is_multiple_whitespace = false;
        let mut is_last_whitespace = false;
        let ver_buf = &buffer[0..2];
        let ver_str = std::str::from_utf8(ver_buf).expect("Couldn't parse header.");
        let is_p6 = ver_str == "P6";
        let mut div_count = 0;
        const HEADER_DIVS: i32 = 5;

        buffer.retain(|val| {
            if is_p6 && div_count >= HEADER_DIVS {
                return true;
            }

            if *val == b'#' {
                is_commented = true;
            }

            if (*val as char).is_whitespace() {
                if !is_last_whitespace && !is_commented {
                    div_count += 1;
                }

                if is_last_whitespace {
                    is_multiple_whitespace = true;
                }

                is_last_whitespace = true;
            } else {
                is_last_whitespace = false;
                is_multiple_whitespace = false;
            }

            let should_retain = (!is_commented.clone() && !is_multiple_whitespace.clone())
                || (is_p6 && div_count >= HEADER_DIVS);
            if *val == b'\n' {
                is_commented = false;
            }

            return should_retain;
        });

        let header_string = get_header_string(buffer);
        self.ver = match header_string.as_str() {
            "P3" => Ok(PPMVer::P3),
            "P6" => Ok(PPMVer::P6),
            _ => Err("Invalid ppm header version."),
        }?;

        let width_string = get_header_string(buffer);
        self.width = width_string.parse().expect("Invalid width parameter.");

        let height_string = get_header_string(buffer);
        self.height = height_string.parse().expect("Invalid height parameter.");

        let max_value_string = get_header_string(buffer);
        self.max_value = max_value_string
            .parse()
            .expect("Invalid max value parameter.");

        let mut u16_buffer = Vec::new();
        if self.ver == PPMVer::P3 {
            let mut num_string = String::new();
            for val in buffer {
                if (*val as char).is_whitespace() {
                    if num_string.len() > 0 {
                        let num: u16 = num_string.parse().expect("Invalid number.");
                        u16_buffer.push(num);
                        num_string.clear();
                    }
                } else {
                    num_string.push(*val as char);
                }
            }
        } else {
            u16_buffer = buffer.iter().map(|val| *val as u16).collect();
        }

        if self.max_value <= u8::MAX.into() {
            self.buffer = BitmapData::U8(u16_buffer.iter().map(|val| *val as u8).collect());
        } else {
            self.buffer = BitmapData::U16(u16_buffer);
        }

        return Ok(());
    }

    pub fn get_max_value(&self) -> usize {
        self.max_value
    }
}

impl Image for PPM {
    fn get_buffer_ref(&self) -> &BitmapData {
        &self.buffer
    }

    fn get_width(&self) -> usize {
        self.width
    }

    fn get_height(&self) -> usize {
        self.height
    }
}

fn get_header_string(vec: &mut Vec<u8>) -> String {
    let header_end = vec
        .iter()
        .position(|val| (*val as char).is_whitespace())
        .expect("Invalid ppm header.");
    let header: Vec<u8> = vec.drain(..header_end + 1).take(header_end).collect();

    return String::from(std::str::from_utf8(&header).expect("Invalid ppm header characters."));
}
