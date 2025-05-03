use crate::downloader::ImageDownloader;
use bytes::Bytes;
use image::{GenericImageView, RgbImage};
use rand::seq::SliceRandom;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use std::fmt::Write as ftmWrite;
use std::io::Write;
use std::time::Duration;
use std::{fmt, io, thread};

#[derive(Debug)]
pub enum ConverterError {
    NoImageLeftError,
    NoImagesRegisteredError,
}

impl fmt::Display for ConverterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConverterError::NoImageLeftError => write!(f, "No images left."),
            ConverterError::NoImagesRegisteredError => write!(f, "No images registered."),
        }
    }
}

struct ColouredImage {
    image_array: Vec<Vec<String>>,
}

impl ColouredImage {
    fn new(image_array: Vec<Vec<String>>) -> Self {
        Self { image_array }
    }

    fn get_random_indices(rows: usize, columns: usize) -> Vec<(usize, usize)> {
        let mut indices: Vec<(usize, usize)> = Vec::with_capacity(rows * columns);
        for row in 0..rows {
            for col in 0..columns {
                indices.push((row, col));
            }
        }
        let mut rng = rand::rng();
        indices.shuffle(&mut rng);
        indices
    }

    fn slow_print(&self) {
        let rows = self.image_array.len();
        let cols = self.image_array[0].len();
        let printing_order = Self::get_random_indices(rows, cols);
        print!("\x1B[2J\x1B[1;1H");
        let empty_row = " ".repeat(cols);
        for _ in 0..rows {
            println!("{}", empty_row);
        }
        print!("\x1B[{}A", rows);
        io::stdout().flush().unwrap();

        for &(row, col) in &printing_order {
            print!("\x1B[{};{}H", row + 1, col + 1);
            print!("{}", self.image_array[row][col]);
            io::stdout().flush().unwrap();
            thread::sleep(Duration::from_millis(5));
        }

        print!("\x1B[{};1H", rows + 1);
        io::stdout().flush().unwrap();
        println!();
    }
    
    fn instant_print(&self) {
        for row in &self.image_array {
            for col in row {
                print!("{}", col);
            }
            println!();       
        }
    }
}

pub struct Converter {
    image_iterator: ImageDownloader,
    coloured_images: Vec<ColouredImage>,
    current_image: usize,
    has_any_images: bool,
    image_width: u32,
}

impl Converter {
    const ASCII_CHARS: [char; 13] = [
        '@', '#', 'S', '%', '&', '?', '*', '+', '-', ':', ',', '.', ' ',
    ];

    pub fn new(image_iterator: ImageDownloader, image_width: u32) -> Self {
        Self {
            image_iterator,
            coloured_images: Vec::new(),
            current_image: 0,
            has_any_images: false,
            image_width,       
        }
    }

    pub fn print_current_image(&self) {
        if !self.has_any_images {
            return;
        }
        if self.current_image == self.coloured_images.len() - 1 {
            self.coloured_images[self.current_image].slow_print();
        }else{
            self.coloured_images[self.current_image].instant_print();
        }
    }

    pub fn move_to_previous_image(&mut self) -> Result<&mut Converter, ConverterError>{
        if !self.has_any_images {
            return Err(ConverterError::NoImagesRegisteredError);       
        }
        if self.current_image == 0 {
            return Err(ConverterError::NoImageLeftError);
        }
        self.current_image -= 1;
        Ok(self)
    }

    pub fn move_to_next_image(&mut self) -> Result<&mut Converter, ConverterError> {
        self.image_iterator
            .next()
            .map(|bytes| self.add_image(bytes))
            .ok_or_else(|| ConverterError::NoImageLeftError)
    }

    fn add_image(&mut self, image_bytes: Bytes) -> &mut Converter {
        if !self.has_any_images {
            self.has_any_images = true;
        }else{
            self.current_image += 1;
        }
        self.coloured_images.push(ColouredImage::new(
            Self::convert_image(image_bytes, self.image_width),
        ));
        self
    }

    fn convert_image(image_bytes: Bytes, image_width: u32) -> Vec<Vec<String>> {
        let img = image::load_from_memory(&image_bytes)
            .expect("This should never fail as image is already in memory");

        let resized: RgbImage = {
            let (original_width, original_height) = img.dimensions();
            let height = original_height * image_width / original_width;
            let height = height.max(1);
            img.resize_exact(
                image_width,
                height,
                image::imageops::FilterType::CatmullRom,
            )
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
