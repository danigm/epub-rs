extern crate xml;

use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use self::xml::reader::EventReader;

use self::xml::reader::XmlEvent as ReaderEvent;
use self::xml::writer::XmlEvent as WriterEvent;

use self::xml::writer::EmitterConfig;
use std::error::Error;
use std::fmt;

use std::borrow::Cow;

// Using RefCell because we need to edit the children vec during the parsing.
// Using rc because a Node will be referenced by its parent and by its childs.
type ChildNodeRef = Rc<RefCell<XMLNode>>;
type ParentNodeRef = Weak<RefCell<XMLNode>>;

pub struct XMLReader<'a> {
    reader: EventReader<&'a [u8]>,
}

impl<'a> XMLReader<'a> {
    pub fn new(content: &[u8]) -> XMLReader {
        XMLReader { reader: EventReader::new(content) }
    }

    pub fn parse_xml(self) -> Result<RefCell<XMLNode>, XMLError> {
        let mut root: Option<ChildNodeRef> = None;
        let mut parents: Vec<ChildNodeRef> = vec![];

        for e in self.reader {
            match e {
                Ok(ReaderEvent::StartElement { name, attributes, namespace }) => {
                    let mut node = XMLNode {
                        name: name,
                        attrs: attributes,
                        namespace: namespace,
                        parent: None,
                        text: None,
                        cdata: None,
                        childs: vec![],
                    };
                    let arnode = Rc::new(RefCell::new(node));

                    {
                        let current = parents.last();
                        if let Some(c) = current {
                            c.borrow_mut().childs.push(arnode.clone());
                            node.parent = Some(Rc::downgrade(&c));
                        }
                    }
                    parents.push(arnode.clone());

                    if root.is_none() {
                        root = Some(arnode.clone());
                    }
                }
                Ok(ReaderEvent::EndElement { .. }) => {
                    if parents.len() > 0 {
                        parents.pop();
                    }
                }
                Ok(ReaderEvent::Characters(text)) => {
                    let current = parents.last();
                    if let Some(c) = current {
                        c.borrow_mut().text = Some(text);
                    }
                }
                Ok(ReaderEvent::CData(text)) => {
                    let current = parents.last();
                    if let Some(c) = current {
                        c.borrow_mut().cdata = Some(text);
                    }
                }
                _ => continue,
            }
        }

        if let Some(r) = root {
            let a = Rc::try_unwrap(r);
            match a {
                Ok(n) => return Ok(n),
                Err(_) => return Err(XMLError { error: String::from("Unknown error") }),
            }
        }
        Err(XMLError { error: String::from("Not xml elements") })
    }
}

#[derive(Debug)]
pub struct XMLError {
    pub error: String,
}

impl Error for XMLError {
    fn description(&self) -> &str {
        &self.error
    }
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
    pub text: Option<String>,
    pub cdata: Option<String>,
    pub parent: Option<ParentNodeRef>,
    pub childs: Vec<ChildNodeRef>,
}

impl XMLNode {
    pub fn get_attr(&self, name: &str) -> Result<String, XMLError> {
        for attr in self.attrs.iter() {
            if attr.name.local_name == name {
                return Ok(attr.value.to_string());
            }
        }

        Err(XMLError { error: String::from("attr not found") })
    }

    pub fn find(&self, tag: &str) -> Result<ChildNodeRef, XMLError> {
        for c in self.childs.iter() {
            if c.borrow().name.local_name == tag {
                return Ok(c.clone());
            } else {
                match c.borrow().find(tag) {
                    Ok(n) => return Ok(n),
                    _ => {}
                }
            }
        }
        Err(XMLError { error: String::from("tag not found") })
    }
}

impl fmt::Display for XMLNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let childs: String = self.childs.iter().fold(String::from(""), |sum, x| {
            sum + &format!("{}", *x.borrow()) + "\n\t"
        });
        let attrs: String =
            self.attrs.iter().fold(String::from(""), |sum, x| sum + &x.name.local_name + ", ");

        let t = self.text.as_ref();
        let mut text = String::from("");
        if let Some(t) = t {
            text.clone_from(t);
        }

        write!(f,
               "<{} [{}]>\n\t{}{}",
               self.name.local_name,
               attrs,
               childs,
               text)
    }
}

// removing &nbsp; from xml
fn clean_xml(xmldoc: &[u8]) -> Vec<u8> {
    String::from_utf8_lossy(xmldoc).replace("&nbsp;", " ").into_bytes()
}

pub fn replace_attrs<F>(xmldoc: &[u8], closure: F) -> Result<Vec<u8>, XMLError>
    where F: Fn(&str, &str, &str) -> String
{
    let mut b = Vec::new();

    {
        let xml_cleaned = clean_xml(xmldoc);
        let reader = EventReader::new(xml_cleaned.as_slice());
        let mut writer = EmitterConfig::default().perform_indent(true).create_writer(&mut b);

        for e in reader {
            match e {
                ev @ Ok(ReaderEvent::StartElement { .. }) => {
                    let ev = ev.unwrap();
                    let mut attrs: Vec<xml::attribute::OwnedAttribute> = vec![];

                    if let Some(WriterEvent::StartElement { name, attributes, namespace }) =
                        ev.as_writer_event() {
                        for i in 0..attributes.len() {
                            let mut attr = attributes[i].to_owned();
                            let repl =
                                closure(&name.local_name, &attr.name.local_name, &attr.value);
                            attr.value = repl;
                            attrs.push(attr);
                        }

                        let w = WriterEvent::StartElement {
                            name: name,
                            attributes: Cow::Owned(attrs.iter().map(|x| x.borrow()).collect()),
                            //attributes: attributes,
                            namespace: namespace,
                        };
                        if let Err(_) = writer.write(w) {
                            return Err(XMLError { error: String::from("Problem writting") });
                        }
                    }
                }
                ev @ Ok(_) => {
                    let ev = ev.unwrap();
                    if let Some(e) = ev.as_writer_event() {
                        if let Err(_) = writer.write(e) {
                            return Err(XMLError { error: String::from("Problem writting") });
                        }
                    }
                }
                Err(err) => return Err(XMLError { error: String::from(err.msg()) }),
            }
        }
    }

    Ok(b)
}
