pub struct Logger;

impl Logger {
    pub fn log_error(error: String) {
        println!("ERROR: {}", error);
    }

    pub fn log_info(info: String) {
        println!("INFO: {}", info);
    }

    pub fn log_success(success: String) {
        println!("SUCCESS: {}", success);
    }
}
