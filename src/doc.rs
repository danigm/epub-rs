//! Manages the epub doc.
//!
//! Provides easy methods to navigate througth the epub content, cover,
//! chapters, etc.

extern crate xml;
extern crate regex;

use std::collections::HashMap;
use std::error::Error;
use std::io::BufReader;
use std::fmt;

use self::xml::reader::{EventReader, XmlEvent};

use archive::EpubArchive;

/// Struct to control the epub document
pub struct EpubDoc {
    /// the zip archive
    archive: EpubArchive,

    /// epub spine ids
    spine: Vec<String>,

    /// resource id -> name
    resources: HashMap<String, String>,

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
        let resources: HashMap<String, String> = HashMap::new();

        let container = try!(archive.get_container_file());
        let root_file = try!(get_root_file(container));

        // getting the rootfile base directory
        let re = regex::Regex::new(r"/").unwrap();
        let iter: Vec<&str> = re.split(&root_file).collect();
        let count = iter.len();
        let base_path = iter[count - 2];

        Ok(EpubDoc {
            archive: archive,
            spine: spine,
            resources: resources,
            root_file: root_file.clone(),
            root_base: String::from(base_path) + "/",
            current: 0,
        })
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
}

// TODO: move this to xmlutils.rs

#[derive(Debug)]
struct XMLError { error: String }

impl Error for XMLError {
    fn description(&self) -> &str { &self.error }
}

impl fmt::Display for XMLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "XMLError: {}", self.error)
    }
}

#[derive(Debug)]
struct XMLNode {
    name: xml::name::OwnedName,
    attrs: Vec<xml::attribute::OwnedAttribute>,
    namespace: xml::namespace::Namespace,
}

trait EasyXml {
    fn get_element_by_tag(self, tag: &str) -> Result<XMLNode, XMLError>;
}

impl<'a> EasyXml for EventReader<&'a [u8]> {
    fn get_element_by_tag(self, tag: &str) -> Result<XMLNode, XMLError> {
        for e in self {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, namespace}) => {
                    if name.local_name == tag {
                        return Ok(XMLNode {
                            name: name,
                            attrs: attributes,
                            namespace: namespace });
                    }
                }
                _ => { continue }
            }
        }
        Err(XMLError { error: String::from("Not found") })
    }
}

fn get_root_file(container: Vec<u8>) -> Result<String, Box<Error>> {
    let parser = EventReader::new(container.as_slice());
    let element = try!(parser.get_element_by_tag("rootfile"));
    for attr in element.attrs {
        if attr.name.local_name == "full-path" {
            return Ok(attr.value);
        }
    }

    Err(Box::new(XMLError {
        error: String::from("full-path attr not found in rootfile")
    }))
}
