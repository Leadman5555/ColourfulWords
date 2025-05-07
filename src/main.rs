mod converter;
mod downloader;
mod image_storage;
mod logger;
mod printer;

use crate::converter::Converter;
use crate::downloader::ImageDownloader;
use crate::image_storage::{ImageStorage, ValidImageLoadIterator};
use crate::logger::Logger;
use crate::printer::{Printer, PrinterError, PrinterImageData};
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use dialoguer::{Input, Select};
use std::env;
use std::io;
use std::process::exit;

fn prompt_for_width() -> u32 {
    loop {
        let width_str = prompt_user("Enter image width");
        match width_str.trim().parse::<u32>() {
            Ok(width) => return width,
            Err(_) => Logger::log_error("Invalid width. Please enter a number."),
        }
    }
}

fn prompt_user(prompt: &str) -> String {
    loop {
        match Input::new().with_prompt(prompt).interact_text() {
            Ok(text) => return text,
            Err(e) => {
                Logger::log_error(&format!(
                    "Failed to get user input: {}. Please try again.",
                    e
                ));
            }
        }
    }
}

fn register_valid_downloader() -> ImageDownloader {
    loop {
        let keyword = prompt_user("Enter keyword");
        match ImageDownloader::new(keyword) {
            Ok(downloader) => return downloader,
            Err(error) => Logger::log_error(&error.to_string()),
        }
    }
}

struct Settings {
    save_location: String,
    load_location: String,
}

fn main() -> io::Result<()> {
    let mut settings = Settings {
        save_location: env::current_dir()?.to_str().unwrap().to_string(),
        load_location: env::current_dir()?.to_str().unwrap().to_string(),
    };
    loop {
        let items = vec![
            "Generator mode",
            "Load saved images",
            "Change settings",
            "Quit",
        ];
        let selection = Select::new()
            .with_prompt("Welcome to Colourful Words!")
            .default(0)
            .items(&items)
            .interact()
            .unwrap();
        match selection {
            0 => {
                match ImageStorage::new(settings.save_location.clone()) {
                    Ok(image_storage) => {
                        let downloader: ImageDownloader = register_valid_downloader();
                        let mut printer: Printer<Converter> =
                            Printer::new(Converter::new(downloader, prompt_for_width()));
                        printer_menu(&create_generator_menu(), &mut printer, &image_storage)?;
                    }
                    Err(e) => Logger::log_error(&e.to_string()),
                }
            }
            1 => {
                match ImageStorage::new(settings.save_location.clone()) {
                    Ok(image_storage) => {
                        match image_storage.to_load_iterator(settings.load_location.as_str()) {
                            Ok(img_loader) => {
                                let mut printer: Printer<ValidImageLoadIterator> =
                                    Printer::new(img_loader.wrap_into_valid());
                                printer_menu(&create_load_menu(), &mut printer, &image_storage)?;
                            }
                            Err(e) => Logger::log_error(&e.to_string()),
                        }
                    }
                    Err(e) => Logger::log_error(&e.to_string()),
                }
            }
            2 => {
                settings_menu(&mut settings);
            }
            3 => {
                exit(0);
            }
            _ => unreachable!(),
        }
    }
}

fn settings_menu(settings: &mut Settings) {
    let items = vec![
        "Change image save location",
        "Change image loading location",
        "Go back",
    ];
    let selection = Select::new()
        .with_prompt("Settings")
        .default(0)
        .items(&items)
        .interact()
        .unwrap();
    match selection {
        0 => {
            let new_location =
                prompt_user("Enter new saving directory path (it must already exist)");
            settings.save_location = new_location;
            Logger::log_info(
                format!("Saving location changed to: {}", settings.save_location).as_str(),
            );
        }
        1 => {
            let new_location =
                prompt_user("Enter new loading directory path (it must already exist)");
            settings.load_location = new_location;
            Logger::log_info(
                format!("Loading location changed to: {}", settings.load_location).as_str(),
            );
        }
        2 => {
            return;
        }
        _ => unreachable!(),
    }
}

struct MenuInfo<G>
where
    G: Iterator<Item = PrinterImageData>,
{
    handle_key_press: fn(KeyCode, image_storage: &ImageStorage, printer: &mut Printer<G>) -> bool,
    print_info: fn() -> (),
}

fn printer_menu<G>(
    menu_info: &MenuInfo<G>,
    printer: &mut Printer<G>,
    image_storage: &ImageStorage,
) -> io::Result<()>
where
    G: Iterator<Item = PrinterImageData>,
{
    (menu_info.print_info)();
    loop {
        if event::poll(std::time::Duration::from_millis(500))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.kind == KeyEventKind::Press {
                    if !(menu_info.handle_key_press)(key_event.code, image_storage, printer) {
                        return Ok(());
                    }
                }
            }
        }
    }
}

fn create_load_menu() -> MenuInfo<ValidImageLoadIterator> {
    MenuInfo {
        handle_key_press: load_menu_handler,
        print_info: || -> () {
            println!("Press 'B' to go back to previous image or 'N' to swap to the next one.");
            println!("Press 'Q' to quit the mode.");
        },
    }
}

fn handle_and_print<G>(res: Result<&mut Printer<G>, PrinterError>) 
where G: Iterator<Item = PrinterImageData>{
    res.map_or_else(
        |e| Logger::log_error(e.to_string().as_str()),
        |printer| -> () {
            let res = printer.print_current_image();
            if res.is_err() {
                Logger::log_error(res.err().unwrap().to_string().as_str());
            }
        },
    )
}

fn load_menu_handler(
    code: KeyCode,
    _: &ImageStorage,
    printer: &mut Printer<ValidImageLoadIterator>,
) -> bool {
    match code {
        KeyCode::Char('b') | KeyCode::Char('B') => {
            handle_and_print(printer.move_to_previous_image());
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            handle_and_print(printer.move_to_next_image());
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            return false;
        }
        _ => {}
    }
    true
}

fn create_generator_menu() -> MenuInfo<Converter> {
    MenuInfo {
        handle_key_press: generator_menu_handler,
        print_info: || -> () {
            println!("Press 'B' to go back to previous image or 'N' to swap to the next one.");
            println!("Press 'S' to save the current image in the specified folder.");
            println!("Press 'Q' to quit the mode.");
        },
    }
}

fn generator_menu_handler(
    code: KeyCode,
    image_storage: &ImageStorage,
    printer: &mut Printer<Converter>,
) -> bool {
    match code {
        KeyCode::Char('b') | KeyCode::Char('B') => {
            handle_and_print(printer.move_to_previous_image());
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            handle_and_print(printer.move_to_next_image());
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let current_image = printer.get_current_image_data();
            if current_image.is_err() {
                Logger::log_error(current_image.err().unwrap().to_string().as_str());
            } else {
                let (image_name, image_array) = current_image.unwrap();
                image_storage
                    .save_image(image_name, image_array)
                    .map_or_else(
                        |e| Logger::log_error(e.to_string().as_str()),
                        |image_name| -> () {
                            Logger::log_success(format!(
                                "Image {} saved successfully.",
                                image_name
                            ).as_str());
                        },
                    )
            }
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            return false;
        }
        _ => {}
    }
    true
}
