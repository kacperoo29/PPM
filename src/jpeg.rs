use std::{error::Error, io::Cursor};

use image::{codecs::jpeg, io::Reader};

use crate::image::{BitmapData, Image};

pub struct JPEG {
    width: u32,
    height: u32,
    data: BitmapData,
}

impl JPEG {
    pub fn from_buffer(buffer: &mut Vec<u8>) -> Self {
        let mut jpeg = JPEG {
            width: 0,
            height: 0,
            data: BitmapData::None,
        };

        jpeg.populate_from_buffer(buffer)
            .expect("Couldn't parse jpeg file.");

        return jpeg;
    }

    pub fn populate_from_buffer(&mut self, buffer: &mut Vec<u8>) -> Result<(), Box<dyn Error>> {
        let mut reader = Reader::new(Cursor::new(&buffer[..]));
        reader.set_format(image::ImageFormat::Jpeg);
        let image = reader.decode()?;

        self.width = image.width();
        self.height = image.height();
        self.data = BitmapData::U8(image.to_rgb8().into_raw());

        Ok(())
    }
}

impl Image for JPEG {
    fn get_width(&self) -> usize {
        self.width as usize
    }

    fn get_height(&self) -> usize {
        self.height as usize
    }

    fn get_buffer_ref(&self) -> &BitmapData {
        &self.data
    }
}
