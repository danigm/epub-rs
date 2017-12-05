//! Manages the zip component part of the epub doc.
//!
//! Provides easy methods to navigate througth the epub parts and to get
//! the content as string.

extern crate zip;

use std::fs;
use std::path;
use failure::Error;

use std::io::Read;

/// Epub archive struct. Here it's stored the file path and the list of
/// files in the zip archive.
pub struct EpubArchive {
    zip: zip::ZipArchive<fs::File>,
    pub path: String,
    pub files: Vec<String>,
}

impl EpubArchive {
    /// Opens the epub file in `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the zip is broken or if the file doesn't
    /// exists.
    pub fn new(path: &str) -> Result<EpubArchive, Error> {
        let fname = path::Path::new(path);
        let file = fs::File::open(&fname)?;

        let mut zip = zip::ZipArchive::new(file)?;
        let mut files = vec![];

        for i in 0..(zip.len()) {
            let file = zip.by_index(i)?;
            files.push(String::from(file.name()));
        }

        Ok(EpubArchive {
            zip: zip,
            path: String::from(path),
            files: files,
        })
    }

    /// Returns the content of the file by the `name` as `Vec<u8>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry(&mut self, name: &str) -> Result<Vec<u8>, Error> {
        let mut entry: Vec<u8> = vec![];
        let mut zipfile = self.zip.by_name(name)?;
        zipfile.read_to_end(&mut entry)?;
        Ok(entry)
    }

    /// Returns the content of the file by the `name` as `String`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry_as_str(&mut self, name: &str) -> Result<String, Error> {
        let mut entry = String::new();
        let mut zipfile = self.zip.by_name(name)?;
        zipfile.read_to_string(&mut entry)?;
        Ok(entry)
    }

    /// Returns the content of container file "META-INF/container.xml".
    ///
    /// # Errors
    ///
    /// Returns an error if the epub doesn't have the container file.
    pub fn get_container_file(&mut self) -> Result<Vec<u8>, Error> {
        let content = self.get_entry("META-INF/container.xml")?;
        Ok(content)
    }
}
