extern crate xml;

use std::cell::RefCell;
use std::sync::Arc;
use std::sync::Weak;
use self::xml::reader::{EventReader, XmlEvent};
use std::error::Error;
use std::fmt;

type ChildNodeRef = Arc<RefCell<XMLNode>>;
type ParentNodeRef = Weak<RefCell<XMLNode>>;

pub struct XMLReader<'a> {
    reader: EventReader<&'a [u8]>
}

impl<'a> XMLReader<'a> {
    pub fn new(content: &[u8]) -> XMLReader {
        XMLReader { reader: EventReader::new(content) }
    }

    pub fn parse_xml(self) -> Option<RefCell<XMLNode>> {
        let mut root: Option<ChildNodeRef> = None;
        let mut parents: Vec<ChildNodeRef> = vec!();

        for e in self.reader {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
                    let mut node = XMLNode {
                        name: name,
                        attrs: attributes,
                        namespace: namespace,
                        parent: None,
                        childs: vec!(),
                    };
                    let mut arnode = Arc::new(RefCell::new(node));

                    {
                        let mut current = parents.last();
                        if current.is_some() {
                            let c = current.unwrap();
                            c.borrow_mut().childs.push(arnode.clone());
                            node.parent = Some(Arc::downgrade(&c));
                        }
                    }
                    parents.push(arnode.clone());

                    if root.is_none() {
                        root = Some(arnode.clone());
                    }
                }
                Ok(XmlEvent::EndElement { name }) => {
                    if parents.len() > 0 {
                        parents.pop();
                    }
                }
                _ => continue
            }
        }

        if root.is_some() {
            let r = root.unwrap();
            let a = Arc::try_unwrap(r);
            match a {
                Ok(n) => return Some(n),
                Err(_) => return None
            }
        }
        None
    }

    pub fn get_element_by_tag(self, tag: &str) -> Result<XMLNode, XMLError> {
        for e in self.reader {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
                    if name.local_name == tag {
                        return Ok(XMLNode {
                            name: name,
                            attrs: attributes,
                            namespace: namespace,
                            parent: None,
                            childs: vec!(),
                        });
                    }
                }
                _ => { continue }
            }
        }
        Err(XMLError { error: String::from("Not found") })
    }

    pub fn get_elements_by_tag(self, tag: &str) -> Vec<XMLNode> {
        let mut elements: Vec<XMLNode> = vec!();
        for e in self.reader {
            match e {
                Ok(XmlEvent::StartElement { name, attributes, namespace }) => {
                    if name.local_name == tag {
                        let node = XMLNode {
                            name: name,
                            attrs: attributes,
                            namespace: namespace,
                            parent: None,
                            childs: vec!(),
                        };
                        elements.push(node);
                    }
                }
                _ => { continue }
            }
        }
        elements
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
    pub parent: Option<ParentNodeRef>,
    pub childs: Vec<ChildNodeRef>,
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
