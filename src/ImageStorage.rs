use std::{fmt, io};
use std::fs::{DirEntry, File, ReadDir};
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum StorageError {
    SavePathError,
    SaveError,
    LoadError(String),
    NotADirError,
    NoImagesFoundError,
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
            StorageError::NoImagesFoundError => write!(f, "No valid images found in the given directory"),
            StorageError::OpeningDirError => write!(f, "Failed to open the given directory"),
            StorageError::IoError(err) => write!(f, "IO error: {}", err),
        }
    }
}

struct ImageStorage {
   save_path: String
}

impl ImageStorage {

    const IMAGE_EXTENSION: &'static str = ".cwi";
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

    pub fn save_image(&self, image_name: String,image_array: Vec<Vec<String>>) -> Result<String, StorageError> {
        let path = Path::new(&self.save_path);
        let path = path.join(image_name + ImageStorage::IMAGE_EXTENSION);
        let mut file: File = File::create::<&Path>(path.as_ref()).map_err(|_| StorageError::SaveError)?;
        for row in image_array {
            std::io::Write::write(&mut file, row.join(Self::CELL_SEPARATOR).as_bytes()).map_err(|_| StorageError::SaveError)?;
        }
        Ok(path.to_str().unwrap().to_string())
    }

    pub fn to_load_iterator(&self, load_path: String) -> Result<ImageLoadIterator, StorageError> {
        ImageLoadIterator::new(load_path)
    }
    
}

struct ImageLoadIterator{
    dir_iter: ReadDir
}

impl ImageLoadIterator {
    pub fn new(load_path: String) -> Result<Self, StorageError> {
        let path = Path::new(&load_path);
        if !path.is_dir() {
            return Err(StorageError::NotADirError);
        }
        Ok(Self{
            dir_iter: path.read_dir().map_err(|_| StorageError::OpeningDirError)?
        })
    }

    fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
    where P: AsRef<Path>, {
        let file = File::open(filename)?;
        Ok(io::BufReader::new(file).lines())
    }

    fn load_image(image_path: PathBuf) -> Result<Vec<Vec<String>>, StorageError> {
        let mut lines = Self::read_lines(&image_path)?;
        let mut result = Vec::new();
        let first_line = lines.next();
        if first_line.is_none() {
            return Err(StorageError::LoadError(image_path.to_str().unwrap().to_string()));
        }
        let first_line = first_line.unwrap()?;
        let expected_length: usize = first_line.len();
        result.push(
            first_line
                .split(ImageStorage::CELL_SEPARATOR)
                .map(str::to_string)
                .collect(),
        );
        for line_result in lines {
            let line = line_result?;
            let current_len = line.len();
            if current_len != expected_length {
                return Err(StorageError::LoadError(image_path.to_str().unwrap().to_string()));
            }
            result.push(
                line.split(ImageStorage::CELL_SEPARATOR)
                    .map(str::to_string)
                    .collect(),
            );
        }
        Ok(result)
    }
}

impl Iterator for ImageLoadIterator {
    type Item = Result<Vec<Vec<String>>, StorageError>;

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
