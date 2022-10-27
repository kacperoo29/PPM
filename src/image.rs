use std::io::Cursor;

use image::{codecs::jpeg::JpegEncoder, DynamicImage, ImageBuffer, ImageResult};

pub enum BitmapData {
    U8(Vec<u8>),
    U16(Vec<u16>),
    None,
}
pub trait Image {
    fn get_width(&self) -> usize;
    fn get_height(&self) -> usize;
    fn get_buffer_ref(&self) -> &BitmapData;

    fn get_pixel_value(&self, x: usize, y: usize) -> (u16, u16, u16) {
        let index = (y * self.get_width() + x) * 3;
        // Guard
        if index >= self.get_width() * self.get_height() * 3 {
            return (0, 0, 0);
        }

        if let BitmapData::U8(data) = self.get_buffer_ref() {
            return (
                data[index] as u16,
                data[index + 1] as u16,
                data[index + 2] as u16,
            );
        } else if let BitmapData::U16(data) = self.get_buffer_ref() {
            return (data[index], data[index + 1], data[index + 2]);
        }

        return (0, 0, 0);
    }

    fn write_to_jpeg(&self, vec: &mut Vec<u8>, quality: u8) -> ImageResult<()> {
        let cursor = Cursor::new(vec);
        let mut encoder = JpegEncoder::new_with_quality(cursor, quality);

        let img = match self.get_buffer_ref() {
            BitmapData::U8(data) => DynamicImage::ImageRgb8(
                ImageBuffer::from_raw(
                    self.get_width() as u32,
                    self.get_height() as u32,
                    data.clone(),
                )
                .unwrap(),
            ),
            BitmapData::U16(data) => DynamicImage::ImageRgb16(
                ImageBuffer::from_raw(
                    self.get_width() as u32,
                    self.get_height() as u32,
                    data.clone(),
                )
                .unwrap(),
            ),
            BitmapData::None => panic!("No data"),
        };

        encoder.encode_image(&img)?;

        Ok(())
    }
}
