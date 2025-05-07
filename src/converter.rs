use crate::downloader::ImageDownloader;
use crate::logger::Logger;
use crate::printer::PrinterImageData;
use bytes::Bytes;
use image::{GenericImageView, RgbImage};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use std::fmt;
use std::fmt::Write;
use std::rc::Rc;

#[derive(Debug)]
pub enum ConverterError {
    ImageLoadingError,
}

impl fmt::Display for ConverterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConverterError::ImageLoadingError => write!(f, "Failed to load image from memory"),
        }
    }
}

pub struct Converter {
    image_iterator: ImageDownloader,
    image_width: u32,
}

impl Converter {
    const ASCII_CHARS: [char; 13] = [
        '@', '#', 'S', '%', '&', '?', '*', '=', '+', '-', ':', ',', '.',
    ];

    pub fn new(image_iterator: ImageDownloader, image_width: u32) -> Self {
        Self {
            image_iterator,
            image_width,
        }
    }

    fn convert_image(
        image_width: u32,
        image_name: Rc<String>,
        image_bytes: Bytes,
    ) -> Result<PrinterImageData, ConverterError> {
        let img =
            image::load_from_memory(&image_bytes).map_err(|_| ConverterError::ImageLoadingError)?;
        let resized: RgbImage = {
            let (original_width, original_height) = img.dimensions();
            let height = original_height * image_width / original_width;
            let height = height.max(1);
            img.resize_exact(image_width, height, image::imageops::FilterType::CatmullRom)
                .to_rgb8()
        };
        let width = resized.width();
        let height = resized.height();
        let ascii_length_m1 = (Self::ASCII_CHARS.len() - 1) as u32;
        let converted_image: Vec<Vec<String>> = (0..height)
            .into_par_iter()
            .map(|y| {
                let mut image_row = vec![String::with_capacity(32); width as usize];
                for x in 0..width {
                    let pixel = resized.get_pixel(x, y);
                    let [r, g, b] = pixel.0;
                    let brightness = (r as u32 + g as u32 + b as u32) / 3;
                    let char_index = ((brightness * ascii_length_m1) + 127) / 255;
                    write!(
                        &mut image_row[x as usize],
                        "\x1B[38;2;{};{};{}m{}\x1B[0m",
                        r,
                        g,
                        b,
                        Self::ASCII_CHARS[char_index as usize]
                    )
                    .expect("Writing to String should not fail");
                }
                image_row
            })
            .collect();
        Ok(PrinterImageData::new(image_name, converted_image))
    }
}

impl Iterator for Converter {
    type Item = PrinterImageData;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.image_iterator.next() {
                Some(image_data_result) => {
                    let (image_name, image_bytes) = image_data_result;
                    match Self::convert_image(self.image_width, image_name.clone(), image_bytes) {
                        Ok(printer_image_data) => return Some(printer_image_data),
                        Err(e) => {
                            Logger::log_error(format!(
                                "Failed to convert image '{}': {}",
                                image_name, e
                            ).as_str());
                        }
                    }
                }
                None => return None,
            }
        }

    }
}
