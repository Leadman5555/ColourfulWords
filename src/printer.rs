use copypasta::{ClipboardContext, ClipboardProvider};
use crossterm::style::Print;
use crossterm::terminal::{Clear, ClearType};
use crossterm::{cursor, QueueableCommand};
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
    ClipboardError,
    InvalidImageError,
}

impl fmt::Display for PrinterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PrinterError::NoImageLeftError => write!(f, "No images left."),
            PrinterError::NoImagesRegisteredError => write!(f, "No images registered."),
            PrinterError::IoError(e) => write!(f, "IO Error during print: {}", e),
            PrinterError::EmptyImageError => write!(f, "Cannot print an empty image."),
            PrinterError::ClipboardError => write!(f, "Failed to copy the current image to clipboard."),
            PrinterError::InvalidImageError => write!(f, "Image contains invalid sequences of characters."),
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
    printing_rate_ms: u16
}

impl ColouredImage {
    fn new(image_array: Vec<Vec<String>>, index: usize, image_name: &Rc<String>, printing_rate_ms: u16) -> Self {
        Self {
            image_array,
            index,
            image_name: image_name.clone(),
            is_rendered: false,
            printing_rate_ms,
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
        let mut stdout = io::stdout();
        let rows = self.image_array.len();
        let cols = self.image_array[0].len();
        let printing_order = Self::get_random_indices(rows, cols);
        stdout.queue(cursor::Hide)?.queue(Clear(ClearType::All))?.queue(cursor::MoveTo(0, 0))?.flush()?;
        let empty_row = " ".repeat(cols);
        for _ in 0..rows {
            stdout.queue(Print(&empty_row))?;
        }
        stdout.queue(cursor::MoveTo(0, 0))?.flush()?;
        for &(row, col) in &printing_order {
            stdout
                .queue(cursor::MoveTo(col as u16, row as u16))?
                .queue(Print(&self.image_array[row][col].to_string()))?
                .flush()?;
            thread::sleep(Duration::from_millis(self.printing_rate_ms as u64));
        }
        stdout.queue(cursor::MoveTo(0, rows as u16))?
            .queue(Print('\n'))?
            .queue(cursor::Show)?
            .flush()?;
        Ok(())
    }

    fn instant_print(&self) -> Result<(), PrinterError> {
        let mut stdout = io::stdout();
        stdout.queue(Clear(ClearType::All))?.queue(cursor::MoveTo(0, 0))?.flush()?;
        for row in &self.image_array {
            stdout.queue(Print(&row.join("")))?.queue(Print('\n'))?.flush()?;
        }
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

    fn get_clipboard_version(&self) -> Result<String, PrinterError> {
        if self.image_array.is_empty() || self.image_array[0].is_empty() {
            return Err(PrinterError::EmptyImageError);
        }
        let mut result = String::with_capacity(self.image_array.len() * (self.image_array[0].len() + 1) + 1);
        for row in &self.image_array {
            row.into_iter().try_for_each(|cell|
                return match cell[..cell.len() - 2].rfind('m') { //..m{CHAR}\..
                Some(backslash_index) => {
                    if backslash_index < cell.len() - 2 {
                        result.push_str(&cell[backslash_index +1.. backslash_index + 2]);
                        Ok(())
                    } else {
                        Err(PrinterError::InvalidImageError)
                    }
                }
                None => Err(PrinterError::InvalidImageError),
            })?;
            result.push('\n');
        }
        if !result.is_empty() {
            result.pop();
        }
        Ok(result)
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
    printing_rate_ms: u16,
}

impl<G> Printer<G>
where
    G: Iterator<Item = PrinterImageData>,
{
    pub fn new(image_generator: G, printing_rate_ms: u16) -> Self {
        Self {
            image_generator,
            coloured_images: Vec::new(),
            current_image: 0,
            printing_rate_ms,       
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
            self.printing_rate_ms
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

    pub fn copy_current_image_to_clipboard(&mut self) -> Result<(), PrinterError> {
        if self.coloured_images.is_empty() {
            return Err(PrinterError::NoImagesRegisteredError);
        }
        let mut clip_ctx = ClipboardContext::new()
            .map_err(|_| PrinterError::ClipboardError)?;
        clip_ctx.set_contents(self.coloured_images[self.current_image].get_clipboard_version()?)
            .map_err(|_| PrinterError::ClipboardError)?;
        Ok(())
    }
    
    #[allow(dead_code)]
    pub fn set_printing_rate(&mut self, printing_rate_ms: u16) {
        self.printing_rate_ms = printing_rate_ms;
    }
}
