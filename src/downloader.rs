use headless_chrome::{Browser, LaunchOptionsBuilder};
use reqwest::blocking;
use std::fmt;
use std::fmt::Debug;
use std::rc::Rc;

#[derive(Debug)]
pub enum DownloaderError {
    ConnectionError,
    NoResultsError,
    BrowserError,
    SearcherError,
}

impl fmt::Display for DownloaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DownloaderError::ConnectionError => write!(f, "Failed to connect to the internet"),
            DownloaderError::NoResultsError => write!(f, "No results found for the given keyword"),
            DownloaderError::BrowserError => write!(f, "Failed to initialize browser"),
            DownloaderError::SearcherError => write!(f, "Failed to search for given keyword"),
        }
    }
}

impl std::error::Error for DownloaderError {}

pub struct ImageDownloader {
    urls: Vec<String>,
    index: usize,
    client: blocking::Client,
    keyword: Rc<String>,   
}

impl ImageDownloader {
    pub fn new(keyword: String) -> Result<Self, DownloaderError> {
        let urls = Self::get_urls(keyword.as_str(), "img.mimg")?;
        Ok(Self {
            urls,
            index: 0,
            client: blocking::Client::default(),
            keyword: Rc::new(keyword),
        })
    }

    fn get_search_url(keyword: &str) -> String {
        format!("{}{}", "https://www.bing.com/images/search?q=", keyword)
    }

    fn get_urls(keyword: &str, selector: &str) -> Result<Vec<String>, DownloaderError> {
        let browser = Browser::new(
            LaunchOptionsBuilder::default()
                .headless(true)
                .build()
                .unwrap(),
        )
        .map_err(|_| DownloaderError::BrowserError)?;
        let tab = browser
            .new_tab()
            .map_err(|_| DownloaderError::BrowserError)?;
        tab.navigate_to(Self::get_search_url(keyword).as_str())
            .map_err(|_| DownloaderError::ConnectionError)?;
        tab.wait_until_navigated()
            .map_err(|_| DownloaderError::SearcherError)?;
        let images = tab
            .wait_for_elements(selector)
            .map_err(|_| DownloaderError::NoResultsError)?;
        let mut results: Vec<String> = Vec::new();
        for img in images {
            if let Some(attr) = img.attributes {
                if let Some(src_attr) = attr.iter().find(|elem| elem.starts_with("https://")) {
                    results.push(src_attr.to_string());
                }
            }
        }
        if results.is_empty() {
            return Err(DownloaderError::NoResultsError);
        }
        Ok(results)
    }
}

impl Iterator for ImageDownloader {
    type Item = (Rc<String>, bytes::Bytes);

    fn next(&mut self) -> Option<Self::Item> {
        while self.index < self.urls.len() {
            let tmp = self.index;
            self.index += 1;
            if let Ok(res) = self.client.get(self.urls[tmp].as_str()).send() {
                if let Ok(bytes) = res.bytes() {
                    return Some((self.keyword.clone(), bytes));
                }
            }
        }
        None
    }
}
