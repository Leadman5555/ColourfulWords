use crate::downloader::ImageDownloader;
use bytes::Bytes;
use image::{GenericImageView, RgbImage};
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use std::fmt::Write as ftmWrite;

pub struct Converter {
    image_iterator: ImageDownloader,
    image_width: u32,
}

impl Converter {
    const ASCII_CHARS: [char; 13] = [
        '@', '#', 'S', '%', '&', '?', '*', '=' ,'+', '-', ':', ',', '.',
    ];

    pub fn new(image_iterator: ImageDownloader, image_width: u32) -> Self {
        Self {
            image_iterator,
            image_width,
        }
    }

    fn convert_image(image_bytes: Bytes, image_width: u32) -> Vec<Vec<String>> {
        let img = image::load_from_memory(&image_bytes)
            .expect("This should never fail as image is already in memory");
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
        converted_image
    }
}

impl Iterator for Converter {
    type Item = Vec<Vec<String>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.image_iterator
            .next()
            .map(|image_bytes| Self::convert_image(image_bytes, self.image_width))
    }
}
