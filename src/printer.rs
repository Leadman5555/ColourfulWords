use rand::prelude::SliceRandom;
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;
use std::{fmt, io, thread};

#[derive(Debug)]
pub enum PrinterError {
    NoImageLeftError,
    NoImagesRegisteredError,
}

impl fmt::Display for PrinterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrinterError::NoImageLeftError => write!(f, "No images left."),
            PrinterError::NoImagesRegisteredError => write!(f, "No images registered."),
        }
    }
}

struct ColouredImage {
    image_array: Vec<Vec<String>>,
    index: usize,
    image_name: Rc<String>,
    is_rendered: bool,
}

impl ColouredImage {
    fn new(image_array: Vec<Vec<String>>, index: usize, image_name: &Rc<String>) -> Self {
        Self {
            image_array,
            index,
            image_name: image_name.clone(),
            is_rendered: false,
        }
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
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush().unwrap();
        for row in &self.image_array {
            for col in row {
                print!("{}", col);
            }
            println!();
        }
        io::stdout().flush().unwrap();
    }

    fn print(&mut self) {
        println!("Image {}", self.index + 1);
        if !self.is_rendered {
            self.slow_print();
            self.is_rendered = true;
        } else {
            self.instant_print();
        }
    }
}

pub struct PrinterImageData {
    image_name: Rc<String>,
    image_array: Vec<Vec<String>>,
}

impl PrinterImageData {
    pub fn new(image_name: Rc<String>, image_array: Vec<Vec<String>>) -> Self {
        Self {
            image_name,
            image_array,
        }
    }
}

pub struct Printer<G>
where
    G: Iterator<Item = PrinterImageData>,
{
    image_generator: G,
    coloured_images: Vec<ColouredImage>,
    current_image: usize,
    has_any_images: bool,
}

impl<G> Printer<G>
where
    G: Iterator<Item = PrinterImageData>,
{
    pub fn new(image_generator: G) -> Self {
        Self {
            image_generator,
            coloured_images: Vec::new(),
            current_image: 0,
            has_any_images: false,
        }
    }

    pub fn get_current_image_data(&self) -> Result<(&str, &Vec<Vec<String>>), PrinterError> {
        if !self.has_any_images {
            return Err(PrinterError::NoImagesRegisteredError);
        }
        let current_image = &self.coloured_images[self.current_image];
        Ok((
            current_image.image_name.as_str(),
            current_image.image_array.as_ref(),
        ))
    }

    pub fn print_current_image(&mut self) {
        if !self.has_any_images {
            return;
        }
        self.coloured_images[self.current_image].print();
    }

    pub fn move_to_previous_image(&mut self) -> Result<&mut Printer<G>, PrinterError> {
        if !self.has_any_images {
            return Err(PrinterError::NoImagesRegisteredError);
        }
        if self.current_image == 0 {
            return Err(PrinterError::NoImageLeftError);
        }
        self.current_image -= 1;
        Ok(self)
    }

    pub fn move_to_next_image(&mut self) -> Result<&mut Printer<G>, PrinterError> {
        if !self.has_any_images || self.current_image == self.coloured_images.len() - 1 {
            self.image_generator.next().map_or_else(
                || Err(PrinterError::NoImageLeftError),
                |image_data| Ok(self.add_image(image_data)),
            )
        } else {
            self.current_image += 1;
            Ok(self)
        }
    }

    fn add_image(&mut self, image_data: PrinterImageData) -> &mut Printer<G> {
        if !self.has_any_images {
            self.has_any_images = true;
        } else {
            self.current_image += 1;
        }
        self.coloured_images
            .push(ColouredImage::new(image_data.image_array, self.current_image, &image_data.image_name));
        self
    }
}
