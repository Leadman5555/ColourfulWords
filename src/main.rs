mod downloader;
mod converter;
mod printer;
mod ImageStorage;

use crate::converter::Converter;
use crate::downloader::ImageDownloader;
use crate::printer::Printer;
use crossterm::event;
use crossterm::event::{Event, KeyCode, KeyEventKind};
use dialoguer::Input;
use std::io;
use std::process::exit;

fn prompt_for_width() -> u32 {
    loop {
        let width = prompt_for_keyword("Enter image width");
        match width.trim().parse::<u32>() {
            Ok(width) => return width,
            Err(_) => println!("Invalid width. Try again."),
        }
    }
}

fn prompt_for_keyword(prompt: &str) -> String {
    Input::new()
        .with_prompt(prompt)
        .interact_text()
        .unwrap()
}

fn register_valid_downloader() -> ImageDownloader {
    let keyword = prompt_for_keyword("Enter keyword");
    ImageDownloader::new(keyword).unwrap_or_else(|error| {
        println!("Error: {}", error);
        register_valid_downloader()
    })
}

fn main() -> io::Result<()>{
    loop {
        let downloader: ImageDownloader = register_valid_downloader();
        let mut printer: Printer = Printer::new(Converter::new(downloader, prompt_for_width()));
        //TODO always have menu visible, swap pictures instead of appending
        println!("Press 'B' to go back to previous image or 'N' to swap to the next one.");
        println!("Press 'Q' to change the keyword or 'ESC' to exit.");
        loop {
            if event::poll(std::time::Duration::from_millis(500))? {
                if let Event::Key(key_event) = event::read()?{
                    if key_event.kind == KeyEventKind::Press {
                        match key_event.code {
                            KeyCode::Char('b') | KeyCode::Char('B') => {
                                printer.move_to_previous_image()
                                    .map_or_else(|e| println!("Error: {}", e), |conv| -> () {
                                        conv.print_current_image();
                                    });
                            },
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                printer.move_to_next_image()
                                    .map_or_else(|e| println!("Error: {}", e), |conv| -> () {
                                        conv.print_current_image();
                                    });
                            },
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                break;
                            },
                            KeyCode::Esc => {
                                exit(0);
                            },
                            _ => {}
                        }
                    }
                }
            }
        }
    }
}