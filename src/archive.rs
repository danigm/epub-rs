//! Manages the zip component part of the epub doc.
//!
//! Provides easy methods to navigate througth the epub parts and to get
//! the content as string.

extern crate percent_encoding;
extern crate zip;

use failure::Error;
use std::fs;
use std::path::{Path, PathBuf};

use std::io::Read;

/// Epub archive struct. Here it's stored the file path and the list of
/// files in the zip archive.
pub struct EpubArchive {
    zip: zip::ZipArchive<fs::File>,
    pub path: PathBuf,
    pub files: Vec<String>,
}

impl EpubArchive {
    /// Opens the epub file in `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the zip is broken or if the file doesn't
    /// exists.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<EpubArchive, Error> {
        let path = path.as_ref();
        let file = fs::File::open(path)?;

        let mut zip = zip::ZipArchive::new(file)?;
        let mut files = vec![];

        for i in 0..(zip.len()) {
            let file = zip.by_index(i)?;
            files.push(String::from(file.name()));
        }

        Ok(EpubArchive {
            zip,
            path: path.to_path_buf(),
            files,
        })
    }

    /// Returns the content of the file by the `name` as `Vec<u8>`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry<P: AsRef<Path>>(&mut self, name: P) -> Result<Vec<u8>, Error> {
        let mut entry: Vec<u8> = vec![];
        let name = name.as_ref().display().to_string();
        match self.zip.by_name(&name) {
            Ok(mut zipfile) => {
                zipfile.read_to_end(&mut entry)?;
                return Ok(entry);
            }
            Err(zip::result::ZipError::FileNotFound) => {}
            Err(e) => {
                return Err(e.into());
            }
        };

        // try percent encoding
        let name =
            self::percent_encoding::percent_decode(name.as_str().as_bytes()).decode_utf8()?;
        let mut zipfile = self.zip.by_name(&name)?;
        zipfile.read_to_end(&mut entry)?;
        Ok(entry)
    }

    /// Returns the content of the file by the `name` as `String`.
    ///
    /// # Errors
    ///
    /// Returns an error if the name doesn't exists in the zip archive.
    pub fn get_entry_as_str<P: AsRef<Path>>(&mut self, name: P) -> Result<String, Error> {
        let content = self.get_entry(name)?;
        String::from_utf8(content).map_err(Error::from)
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
