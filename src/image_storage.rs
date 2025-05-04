use rayon::iter::ParallelIterator;
use crate::logger::Logger;
use crate::printer::PrinterImageData;
use std::fs::{File, ReadDir};
use std::io::{BufRead, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::thread::sleep;
use std::time::SystemTime;
use std::{fmt, io};
use rayon::prelude::ParallelBridge;

#[derive(Debug)]
pub enum StorageError {
    SavePathError,
    SaveError,
    LoadError(String),
    NotADirError,
    OpeningDirError,
    IoError(io::Error)
}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        StorageError::IoError(err)
    }
}


impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StorageError::SavePathError => write!(f, "Given save path is not a valid directory"),
            StorageError::SaveError => write!(f, "Could not save image to the given save directory"),
            StorageError::LoadError(image_name) => write!(f, "Image {image_name} couldn't be loaded"),
            StorageError::NotADirError => write!(f, "Failed to search for given keyword"),
            StorageError::OpeningDirError => write!(f, "Failed to open the given directory"),
            StorageError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

pub struct ImageStorage {
   save_path: String
}

impl ImageStorage {

    const IMAGE_EXTENSION: &'static str = "cwi";
    const CELL_SEPARATOR: &'static str = " ";

    pub fn new(save_path: String) -> Result<Self, StorageError> {
        let path = Path::new(&save_path);
        if !path.is_dir() {
            return Err(StorageError::SavePathError);
        }
        Ok(Self{
            save_path
        })
    }

    fn get_image_name(image_name: &str) -> String {
        format!("{}_{}.{}", SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("This will always be correct")
            .as_secs(),  image_name, Self::IMAGE_EXTENSION)
    }

    pub fn save_image(&self, image_name: &str,image_array: &Vec<Vec<String>>) -> Result<String, StorageError> {
        let path = Path::new(&self.save_path);
        let new_image_name = Self::get_image_name(image_name);
        let mut path = path.join(new_image_name.as_str());
        while path.exists() {
            sleep(std::time::Duration::from_millis(200));
            path = path.join(Self::get_image_name(image_name));
        }
        let mut writer = BufWriter::new(File::create::<&Path>(path.as_ref()).map_err(|_| StorageError::SaveError)?);
        for row in image_array {
            writeln!(writer, "{}", row.join(Self::CELL_SEPARATOR)).map_err(|_| StorageError::SaveError)?;
        }
        writer.flush().map_err(|_| StorageError::SaveError)?;
        Ok(new_image_name)
    }

    pub fn to_load_iterator(&self, load_path: &str) -> Result<ImageLoadIterator, StorageError> {
        ImageLoadIterator::new(load_path)
    }
    
}

pub struct ImageLoadIterator{
    dir_iter: ReadDir
}

impl ImageLoadIterator {
    fn new(load_path: &str) -> Result<Self, StorageError> {
        let path = Path::new(&load_path);
        if !path.is_dir() {
            return Err(StorageError::NotADirError);
        }
        Ok(Self{
            dir_iter: path.read_dir().map_err(|_| StorageError::OpeningDirError)?
        })
    }

    pub fn wrap_into_valid(self) -> ValidImageLoadIterator {
        ValidImageLoadIterator {
            iterator: self,
        }
    }

    fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
        let file = File::open(filename)?;
        Ok(io::BufReader::new(file).lines())
    }

    fn load_image(image_path: PathBuf) -> Result<PrinterImageData, StorageError> {
        let mut lines = Self::read_lines(&image_path)?;
        let path_string = image_path.to_string_lossy().to_string();
        let load_error = || StorageError::LoadError(path_string.clone());
        let first_line: Vec<_> = lines.next().ok_or(load_error())??
            .split(ImageStorage::CELL_SEPARATOR)
            .map(str::to_string)
            .collect();
        let expected_length: usize = first_line.len();
        let mut result = vec![first_line];

        let remaining_lines: Vec<_> = lines
            .par_bridge()
            .map(|line| {
                let current_line: Vec<String> = line?.split(ImageStorage::CELL_SEPARATOR)
                    .map(str::to_string)
                    .collect();

                if current_line.len() != expected_length {
                    Err(load_error())
                } else {
                    Ok(current_line)
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        result.reserve(remaining_lines.len());
        result.extend(remaining_lines);
        Ok(PrinterImageData::new(
            Rc::new(image_path.file_name().expect("Path is already checked to be valid").to_string_lossy().to_string()),
            result,
        ))

    }
}

impl Iterator for ImageLoadIterator {
    type Item = Result<PrinterImageData, StorageError>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(entry) = self.dir_iter.next() {
            let entry = match entry {
                Ok(entry) => entry,
                Err(_) => continue,
            };
            let full_path = entry.path();
            let extension = full_path.extension();
            if extension.is_none() || extension.unwrap() != ImageStorage::IMAGE_EXTENSION {
                continue;
            }
            return Some(ImageLoadIterator::load_image(full_path));
        }
        None
    }
}

pub struct ValidImageLoadIterator{
    iterator: ImageLoadIterator,
}

impl Iterator for ValidImageLoadIterator{
    type Item = PrinterImageData;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.iterator.next() {
                Some(Ok(image)) => return Some(image),
                Some(Err(err)) => Logger::log_error(err.to_string()),
                None => return None
            }
        }
    }

}
