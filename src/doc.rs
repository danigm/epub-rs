//! Manages the epub doc.
//!
//! Provides easy methods to navigate througth the epub content, cover,
//! chapters, etc.

extern crate xml;
extern crate regex;

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

use archive::EpubArchive;

use xmlutils;

#[derive(Debug)]
pub struct DocError { pub error: String }

impl Error for DocError {
    fn description(&self) -> &str { &self.error }
}

impl fmt::Display for DocError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "DocError: {}", self.error)
    }
}

/// Struct to control the epub document
pub struct EpubDoc {
    /// the zip archive
    archive: EpubArchive,

    /// epub spine ids
    pub spine: Vec<String>,

    /// resource id -> name
    pub resources: HashMap<String, (String, String)>,

    /// The epub metadata stored as key -> value
    ///
    /// #Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.metadata.get("title");
    /// assert_eq!(title.unwrap(), "Todo es mío");
    /// ```
    pub metadata: HashMap<String, String>,

    /// The current chapter, is an spine index
    current: u32,

    /// root file base path
    pub root_base: String,

    /// root file full path
    pub root_file: String,
}

impl EpubDoc {
    /// Opens the epub file in `path`.
    ///
    /// Initialize some internal variables to be able to access to the epub
    /// spine definition and to navigate trhough the epub.
    ///
    /// # Errors
    ///
    /// Returns an error if the epub is broken or if the file doesn't
    /// exists.
    pub fn new(path: &str) -> Result<EpubDoc, Box<Error>> {
        let mut archive = try!(EpubArchive::new(path));
        let spine: Vec<String> = vec!();
        let resources: HashMap<String, (String, String)> = HashMap::new();

        let container = try!(archive.get_container_file());
        let root_file = try!(get_root_file(container));

        // getting the rootfile base directory
        let re = regex::Regex::new(r"/").unwrap();
        let iter: Vec<&str> = re.split(&root_file).collect();
        let count = iter.len();
        let base_path = if count >= 2 { iter[count - 2] } else { "" };

        let mut doc = EpubDoc {
            archive: archive,
            spine: spine,
            resources: resources,
            metadata: HashMap::new(),
            root_file: root_file.clone(),
            root_base: String::from(base_path) + "/",
            current: 0,
        };

        try!(doc.fill_resources());

        Ok(doc)
    }

    /// Returns the id of the epub cover.
    ///
    /// The cover is searched in the doc metadata, by the tag <meta name="cover" value"..">
    ///
    /// # Examples
    ///
    /// ```rust
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// let mut doc = doc.unwrap();
    ///
    /// let cover_id = doc.get_cover_id().unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cover path can't be found.
    pub fn get_cover_id(&self) -> Result<String, Box<Error>> {
        match self.metadata.get("cover") {
            Some(id) => Ok(id.to_string()),
            None => Err(Box::new(DocError { error: String::from("Cover not found") }))
        }
    }

    /// Returns the cover as Vec<u8>
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use std::fs;
    /// use std::io::Write;
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// let mut doc = doc.unwrap();
    ///
    /// let cover_data = doc.get_cover().unwrap();
    ///
    /// let f = fs::File::create("/tmp/cover.png");
    /// assert!(f.is_ok());
    /// let mut f = f.unwrap();
    /// let resp = f.write_all(&cover_data);
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the cover can't be found.
    pub fn get_cover(&mut self) -> Result<Vec<u8>, Box<Error>> {
        let cover_id = try!(self.get_cover_id());
        let cover_data = try!(self.get_resource(&cover_id));
        Ok(cover_data)
    }

    /// Returns the resource content by full path in the epub archive
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_by_path(&mut self, path: &str) -> Result<Vec<u8>, Box<Error>> {
        let content = try!(self.archive.get_entry(path));
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine
    ///
    /// # Errors
    ///
    /// Returns an error if the id doesn't exists in the epub
    pub fn get_resource(&mut self, id: &str) -> Result<Vec<u8>, Box<Error>> {
        let path: String = match self.resources.get(id) {
            Some(s) => s.0.to_string(),
            None => return Err(Box::new(DocError { error: String::from("id not found") }))
        };
        let content = try!(self.get_resource_by_path(&path));
        Ok(content)
    }

    /// Returns the resource content by full path in the epub archive, as String
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_str_by_path(&mut self, path: &str) -> Result<String, Box<Error>> {
        let content = try!(self.archive.get_entry_as_str(path));
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine, as String
    ///
    /// # Errors
    ///
    /// Returns an error if the id doesn't exists in the epub
    pub fn get_resource_str(&mut self, id: &str) -> Result<String, Box<Error>> {
        let path: String = match self.resources.get(id) {
            Some(s) => s.0.to_string(),
            None => return Err(Box::new(DocError { error: String::from("id not found") }))
        };
        let content = try!(self.get_resource_str_by_path(&path));
        Ok(content)
    }

    /// Returns the resource mime-type
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let mime = doc.get_resource_mime("portada.png");
    /// assert_eq!("image/png", mime.unwrap());
    /// ```
    /// # Errors
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime(&self, id: &str) -> Result<String, Box<Error>> {
        match self.resources.get(id) {
            Some(&(_, ref res)) => return Ok(res.to_string()),
            None => {}
        }
        Err(Box::new(DocError { error: String::from("id not found") }))
    }

    /// Returns the resource mime searching by source full path
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let mime = doc.get_resource_mime_by_path("OEBPS/Images/portada.png");
    /// assert_eq!("image/png", mime.unwrap());
    /// ```
    /// # Errors
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime_by_path(&self, path: &str) -> Result<String, Box<Error>> {
        for (_, v) in self.resources.iter() {
            if v.0 == path {
                return Ok(v.1.to_string());
            }
        }
        Err(Box::new(DocError { error: String::from("path not found") }))
    }

    //pub fn get_current() -> Result<Vec<u8>, Box<Error>> {}
    //pub fn get_current_str() -> Result<String, Box<Error>> {}

    //pub fn get_current_mime() -> Result<String, Box<Error>> {}

    //pub fn get_current_path() -> Result<String, Box<Error>> {}
    //pub fn get_current_id() -> Result<String, Box<Error>> {}

    //pub fn go_next() -> Result<(), Box<Error>> {}
    //pub fn go_prev() -> Result<(), Box<Error>> {}

    //pub fn get_num_pages() -> u32 {}

    //pub fn get_current_page() -> u32 {}
    //pub fn set_current_page(n: u32) {}

    fn fill_resources(&mut self) -> Result<(), Box<Error>> {
        let container = try!(self.archive.get_entry(&self.root_file));
        let xml = xmlutils::XMLReader::new(container.as_slice());
        let root = try!(xml.parse_xml());

        // resources from manifest
        let manifest = try!(root.borrow().find("manifest"));
        for r in manifest.borrow().childs.iter() {
            let item = r.borrow();
            let id = try!(item.get_attr("id"));
            let href = try!(item.get_attr("href"));
            let mtype = try!(item.get_attr("media-type"));
            self.resources.insert(id, (self.root_base.to_string() + &href, mtype));
        }

        // items from spine
        let spine = try!(root.borrow().find("spine"));
        for r in spine.borrow().childs.iter() {
            let item = r.borrow();
            let id = try!(item.get_attr("idref"));
            self.spine.push(id);
        }

        // metadata
        let metadata = try!(root.borrow().find("metadata"));
        for r in metadata.borrow().childs.iter() {
            let item = r.borrow();
            if item.name.local_name == "meta" {
                let k = try!(item.get_attr("name"));
                let v = try!(item.get_attr("content"));
                self.metadata.insert(k, v);
            } else {
                let ref k = item.name.local_name;
                let v = match item.text { Some(ref x) => x.to_string(), None => String::from("") };
                self.metadata.insert(k.to_string(), v);
            }
        }

        Ok(())
    }
}

fn get_root_file(container: Vec<u8>) -> Result<String, Box<Error>> {
    let xml = xmlutils::XMLReader::new(container.as_slice());
    let root = try!(xml.parse_xml());
    let el = root.borrow();
    let element = try!(el.find("rootfile"));
    let el2 = element.borrow();

    Ok(try!(el2.get_attr("full-path")))
}
