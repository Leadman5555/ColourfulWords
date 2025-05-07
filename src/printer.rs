use rand::prelude::SliceRandom;
use std::io::Write;
use std::rc::Rc;
use std::time::Duration;
use std::{fmt, io, thread};

#[derive(Debug)]
pub enum PrinterError {
    NoImageLeftError,
    NoImagesRegisteredError,
    IoError(io::Error),
    EmptyImageError,
}

impl fmt::Display for PrinterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrinterError::NoImageLeftError => write!(f, "No images left."),
            PrinterError::NoImagesRegisteredError => write!(f, "No images registered."),
            PrinterError::IoError(e) => write!(f, "IO Error during print: {}", e),
            PrinterError::EmptyImageError => write!(f, "Cannot print an empty image."),
        }
    }
}

impl From<io::Error> for PrinterError {
    fn from(err: io::Error) -> Self {
        PrinterError::IoError(err)
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

    fn slow_print(&self) -> Result<(), PrinterError> {
        if self.image_array.is_empty() || self.image_array[0].is_empty() {
            return Err(PrinterError::EmptyImageError);
        }
        let rows = self.image_array.len();
        let cols = self.image_array[0].len();
        let printing_order = Self::get_random_indices(rows, cols);
        print!("\x1B[2J\x1B[1;1H"); //clear, top-left
        let empty_row = " ".repeat(cols);
        for _ in 0..rows {
            println!("{}", empty_row);
        }
        print!("\x1B[{}A", rows); //move to the beginning
        io::stdout().flush()?;

        for &(row, col) in &printing_order {
            print!("\x1B[{};{}H", row + 1, col + 1);
            print!("{}", self.image_array[row][col]);
            io::stdout().flush()?;
            thread::sleep(Duration::from_millis(5));
        }

        print!("\x1B[{};1H", rows + 1); //move down
        io::stdout().flush()?;
        println!();
        Ok(())
    }

    fn instant_print(&self) -> Result<(), PrinterError> {
        print!("\x1B[2J\x1B[1;1H");
        io::stdout().flush()?;
        for row in &self.image_array {
            for col in row {
                print!("{}", col);
            }
            println!();
        }
        io::stdout().flush()?;
        Ok(())
    }

    fn print(&mut self) -> Result<(), PrinterError> {
        println!("Image {}", self.index + 1);
        if !self.is_rendered {
            self.slow_print()?;
            self.is_rendered = true;
        } else {
            self.instant_print()?;
        }
        Ok(())
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
        }
    }

    pub fn get_current_image_data(&self) -> Result<(&str, &Vec<Vec<String>>), PrinterError> {
        if self.coloured_images.is_empty() {
            return Err(PrinterError::NoImagesRegisteredError);
        }
        let current_image = &self.coloured_images[self.current_image];
        Ok((
            current_image.image_name.as_str(),
            &current_image.image_array,
        ))
    }

    pub fn print_current_image(&mut self) -> Result<(), PrinterError>{
        if self.coloured_images.is_empty() {
            if let Some(image_data) = self.image_generator.next() {
                self.add_image_and_set_current(image_data);
            } else {
                return Err(PrinterError::NoImagesRegisteredError);
            }
        }
        self.coloured_images[self.current_image].print()
    }

    fn add_image_and_set_current(&mut self, image_data: PrinterImageData) {
        let new_image_index = self.coloured_images.len();
        self.coloured_images.push(ColouredImage::new(
            image_data.image_array,
            new_image_index,
            &image_data.image_name,
        ));
        self.current_image = new_image_index; 
    }


    pub fn move_to_previous_image(&mut self) -> Result<&mut Printer<G>, PrinterError> {
        if self.coloured_images.is_empty() {
            return Err(PrinterError::NoImagesRegisteredError);
        }
        if self.current_image == 0 {
            return Err(PrinterError::NoImageLeftError);
        }
        self.current_image -= 1;
        Ok(self)
    }

    pub fn move_to_next_image(&mut self) -> Result<&mut Printer<G>, PrinterError> {
        if self.coloured_images.is_empty() {
            return match self.image_generator.next() {
                Some(image_data) => {
                    self.add_image_and_set_current(image_data);
                    Ok(self)
                }
                None => Err(PrinterError::NoImagesRegisteredError)
            }
        }
        if self.current_image < self.coloured_images.len() - 1 {
            self.current_image += 1;
            Ok(self)
        } else {
            match self.image_generator.next() {
                Some(image_data) => {
                    self.add_image_and_set_current(image_data);
                    Ok(self)
                }
                None => Err(PrinterError::NoImageLeftError),
            }
        }

    }
}
