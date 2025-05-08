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
        let width_str = prompt_user("Enter image width (tip: enter 100 and zoom out with CRTL-)");
        match width_str.trim().parse::<u32>() {
            Ok(width) => return width,
            Err(_) => Logger::log_error("Invalid width. Please enter a positive integer."),
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

fn register_valid_printing_rate() -> u16 {
    loop {
        let rate = prompt_user("Enter new printing rate in milliseconds (default is 5 ms)");
        match rate.trim().parse::<u16>() {
            Ok(rate) => return rate,
            Err(_) => Logger::log_error("Invalid printing rate. Please enter an integer [0 - 655635]."),
        }
    }
}

struct Settings {
    save_location: String,
    load_location: String,
    printing_rate_ms: u16
}

const BANNER: &'static str =
"\x1B[38;2;255;0;0mW\x1B[0m\
\x1B[38;2;255;127;0me\x1B[0m\
\x1B[38;2;255;255;0ml\x1B[0m\
\x1B[38;2;0;255;0mc\x1B[0m\
\x1B[38;2;56;241;255mo\x1B[0m\
\x1B[38;2;135;255;219mm\x1B[0m\
\x1B[38;2;148;0;211me\x1B[0m\
\x1B[38;2;255;0;0m \x1B[0m\
\x1B[38;2;255;127;0mt\x1B[0m\
\x1B[38;2;255;255;0mo\x1B[0m\
\x1B[38;2;0;255;0m \x1B[0m\
\x1B[38;2;56;241;255mC\x1B[0m\
\x1B[38;2;135;255;219mo\x1B[0m\
\x1B[38;2;148;0;211ml\x1B[0m\
\x1B[38;2;255;0;0mo\x1B[0m\
\x1B[38;2;255;127;0mu\x1B[0m\
\x1B[38;2;255;255;0mr\x1B[0m\
\x1B[38;2;0;255;0mf\x1B[0m\
\x1B[38;2;56;241;255mu\x1B[0m\
\x1B[38;2;135;255;219ml\x1B[0m\
\x1B[38;2;148;0;211m \x1B[0m\
\x1B[38;2;255;0;0mW\x1B[0m\
\x1B[38;2;255;127;0mo\x1B[0m\
\x1B[38;2;255;255;0mr\x1B[0m\
\x1B[38;2;0;255;0md\x1B[0m\
\x1B[38;2;56;241;255ms\x1B[0m\
\x1B[38;2;135;255;219m!\x1B[0m";


fn main() -> io::Result<()> {
    let mut settings = Settings {
        save_location: env::current_dir()?.to_str().unwrap().to_string(),
        load_location: env::current_dir()?.to_str().unwrap().to_string(),
        printing_rate_ms: 5
    };
    loop {
        let items = vec![
            "Generator mode",
            "Load saved images",
            "Change settings",
            "Quit",
        ];
        let selection = Select::new()
            .with_prompt(BANNER)
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
                            Printer::new(Converter::new(downloader, prompt_for_width()), settings.printing_rate_ms);
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
                                    Printer::new(img_loader.wrap_into_valid(), settings.printing_rate_ms);
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
        "Change image printing rate",
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
            let new_printing_rate = register_valid_printing_rate();
            settings.printing_rate_ms = new_printing_rate;
            Logger::log_info(
                format!("Printing rate changed to: {}", settings.printing_rate_ms).as_str(),
            );
        }
        3 => {
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
            println!("Press 'C' to copy a colourless version of the current image to clipboard.");
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
        KeyCode::Char('C') | KeyCode::Char('c') => {
            printer.copy_current_image_to_clipboard()
                .map_or_else(|e| Logger::log_error(e.to_string().as_str()), |_| Logger::log_success("Image copied to clipboard."));
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
            println!("Press 'C' to copy a colourless version of the current image to clipboard.");
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
        KeyCode::Char('C') | KeyCode::Char('c') => {
            printer.copy_current_image_to_clipboard()
                .map_or_else(|e| Logger::log_error(e.to_string().as_str()), |_| Logger::log_success("Image copied to clipboard."));
        }
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            return false;
        }
        _ => {}
    }
    true
}
