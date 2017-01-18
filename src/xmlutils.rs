extern crate xml;

use self::xml::reader::{EventReader, XmlEvent};
use std::error::Error;
use std::fmt;

pub struct XMLReader<'a> {
    reader: EventReader<&'a [u8]>
}

impl<'a> XMLReader<'a> {
    pub fn new(content: &[u8]) -> XMLReader {
        XMLReader { reader: EventReader::new(content) }
    }

    pub fn get_element_by_tag(self, tag: &str) -> Result<XMLNode, XMLError> {
        for e in self.reader {
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

#[derive(Debug)]
pub struct XMLError { pub error: String }

impl Error for XMLError {
    fn description(&self) -> &str { &self.error }
}

impl fmt::Display for XMLError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "XMLError: {}", self.error)
    }
}

#[derive(Debug)]
pub struct XMLNode {
    pub name: xml::name::OwnedName,
    pub attrs: Vec<xml::attribute::OwnedAttribute>,
    pub namespace: xml::namespace::Namespace,
}

impl XMLNode {
    pub fn get_attr(self, name: &str) -> Result<String, XMLError> {
        for attr in self.attrs {
            if attr.name.local_name == name {
                return Ok(attr.value);
            }
        }

        Err(XMLError { error: String::from("attr not found") })
    }
}
