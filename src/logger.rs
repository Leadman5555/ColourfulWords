pub struct Logger;

impl Logger {
    pub fn log_error(error: &str) {
        eprintln!("ERROR: {}", error);
    }

    pub fn log_info(info: &str) {
        println!("INFO: {}", info);
    }

    pub fn log_success(success: &str) {
        println!("SUCCESS: {}", success);
    }
}
