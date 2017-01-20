//! Manages the epub doc.
//!
//! Provides easy methods to navigate througth the epub content, cover,
//! chapters, etc.

extern crate xml;
extern crate regex;

use std::collections::HashMap;
use std::error::Error;

use archive::EpubArchive;

use xmlutils;

/// Struct to control the epub document
pub struct EpubDoc {
    /// the zip archive
    archive: EpubArchive,

    /// epub spine ids
    pub spine: Vec<String>,

    /// resource id -> name
    pub resources: HashMap<String, (String, String)>,

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
            root_file: root_file.clone(),
            root_base: String::from(base_path) + "/",
            current: 0,
        };

        try!(doc.fill_resources());

        Ok(doc)
    }

    //pub fn get_cover() -> Result<Vec<u8>, Box<Error>> {}
    //pub fn get_metadata(mdata: &str) -> Result<String, Box<Error>> {}

    //pub fn get_resource(path: &str) -> Result<Vec<u8>, Box<Error>> {}
    //pub fn get_resource_str(path: &str) -> Result<String, Box<Error>> {}

    //pub fn get_resource_by_id(id: &str) -> Result<Vec<u8>, Box<Error>> {}
    //pub fn get_resource_by_id_str(id: &str) -> Result<String, Box<Error>> {}

    //pub fn get_resources() -> Result<HashMap<String, String>, Box<Error>> {}

    //pub fn get_resource_mime(path: &str) -> Result<String, Box<Error>> {}
    //pub fn get_resource_mime_by_id(id: &str) -> Result<String, Box<Error>> {}

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
        let manifest = try!(root.borrow().find("manifest"));
        for r in manifest.borrow().childs.iter() {
            let item = r.borrow();
            let id = try!(item.get_attr("id"));
            let href = try!(item.get_attr("href"));
            let mtype = try!(item.get_attr("media-type"));
            self.resources.insert(id, (href, mtype));
        }

        let spine = try!(root.borrow().find("spine"));
        for r in spine.borrow().childs.iter() {
            let item = r.borrow();
            let id = try!(item.get_attr("idref"));
            self.spine.push(id);
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
