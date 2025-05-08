use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::QueueableCommand;
use std::io::Write;

pub struct Logger;

impl Logger {
    fn log_with_colour(colour: Color, message: &str) -> Result<(), std::io::Error> {
        std::io::stdout()
            .queue(SetForegroundColor(colour))?
            .queue(Print(message))?
            .queue(ResetColor)?
            .flush()
    }

    fn log_without_color(message: String) {
        println!("{}", message);
    }

    pub fn log_info(info: &str) {
        let message = format!("INFO: {}\n", info);
        if Self::log_with_colour(Color::Yellow, message.as_str()).is_err() {
            Self::log_without_color(message)
        }
    }

    pub fn log_success(success: &str) {
        let message = format!("Success: {}\n", success);
        if Self::log_with_colour(Color::Green, message.as_str()).is_err() {
            Self::log_without_color(message)
        }
    }

    pub fn log_error(error: &str) {
        let message = format!("ERROR: {}\n", error);
        if Self::log_with_colour(Color::Red, message.as_str()).is_err() {
            Self::log_without_color(message)
        }
    }
}
