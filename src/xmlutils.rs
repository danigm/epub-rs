use std::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;
use xml::reader::Error as ReaderError;
use xml::reader::EventReader;
use xml::reader::ParserConfig;

use xml::reader::XmlEvent as ReaderEvent;
use xml::writer::XmlEvent as WriterEvent;

use std::error::Error;
use std::fmt;
use xml::writer::EmitterConfig;
use xml::writer::Error as EmitterError;

use std::borrow::Cow;

// Using RefCell because we need to edit the children vec during the parsing.
// Using rc because a Node will be referenced by its parent and by its childs.
type ChildNodeRef = Rc<RefCell<XMLNode>>;
type ParentNodeRef = Weak<RefCell<XMLNode>>;

pub struct XMLReader<'a> {
    reader: EventReader<&'a [u8]>,
}

impl<'a> XMLReader<'a> {
    pub fn parse(content: &[u8]) -> Result<RefCell<XMLNode>, XMLError> {
        let content_str;
        //If there is a UTF-8 BOM marker, ignore it
        let content_slice = if content[0..3] == [0xefu8, 0xbbu8, 0xbfu8] {
            &content[3..]
        } else if content[0..2] == [0xfeu8, 0xffu8] || content[0..2] == [0xffu8, 0xfeu8] { //handle utf-16
            let (big_byte, small_byte) = if content[0] == 0xfeu8 {
                (1,0) //big endian utf-16
            } else {
                (0,1) //little endian utf-16
            };
            let content_u16: Vec<u16> = content[2..]
                .chunks_exact(2)
                .into_iter()
                .map(|a| u16::from_ne_bytes([a[big_byte], a[small_byte]]))
                .collect();
            content_str = String::from_utf16_lossy(content_u16.as_slice());
            content_str.as_bytes()
        } else {
            content
        };

        let reader = XMLReader {
            reader: ParserConfig::new()
                .add_entity("nbsp", " ")
                .add_entity("copy", "©")
                .add_entity("reg", "®")
                .create_reader(content_slice)
        };

        reader.parse_xml()
    }

    fn parse_xml(self) -> Result<RefCell<XMLNode>, XMLError> {
        let mut root: Option<ChildNodeRef> = None;
        let mut parents: Vec<ChildNodeRef> = vec![];

        for e in self.reader {
            match e {
                Ok(ReaderEvent::StartElement {
                    name,
                    attributes,
                    namespace,
                }) => {
                    let node = XMLNode {
                        name,
                        attrs: attributes,
                        namespace,
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
                            arnode.borrow_mut().parent = Some(Rc::downgrade(&c));
                        }
                    }
                    parents.push(arnode.clone());

                    if root.is_none() {
                        root = Some(arnode.clone());
                    }
                }
                Ok(ReaderEvent::EndElement { .. }) => {
                    if !parents.is_empty() {
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
                Err(_) => {
                    return Err(XMLError {
                        error: String::from("Unknown error"),
                    })
                }
            }
        }
        Err(XMLError {
            error: String::from("Not xml elements"),
        })
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "XMLError: {}", self.error)
    }
}

impl From<EmitterError> for XMLError {
    fn from(_: EmitterError) -> XMLError {
        XMLError {
            error: String::from("Problem writting"),
        }
    }
}

impl From<ReaderError> for XMLError {
    fn from(_: ReaderError) -> XMLError {
        XMLError {
            error: String::from("Problem reading"),
        }
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

        Err(XMLError {
            error: String::from("attr not found"),
        })
    }

    pub fn find(&self, tag: &str) -> Result<ChildNodeRef, XMLError> {
        for c in self.childs.iter() {
            if c.borrow().name.local_name == tag {
                return Ok(c.clone());
            } else if let Ok(n) = c.borrow().find(tag) {
                return Ok(n);
            }
        }
        Err(XMLError {
            error: String::from("tag not found"),
        })
    }
}

impl fmt::Display for XMLNode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let childs: String = self.childs.iter().fold(String::from(""), |sum, x| {
            sum + &format!("{}", *x.borrow()) + "\n\t"
        });
        let attrs: String = self
            .attrs
            .iter()
            .fold(String::from(""), |sum, x| sum + &x.name.local_name + ", ");

        let t = self.text.as_ref();
        let mut text = String::from("");
        if let Some(t) = t {
            text.clone_from(t);
        }

        write!(
            f,
            "<{} [{}]>\n\t{}{}",
            self.name.local_name, attrs, childs, text
        )
    }
}

pub fn replace_attrs<F>(
    xmldoc: &[u8],
    closure: F,
    extra_css: &[String],
) -> Result<Vec<u8>, XMLError>
where
    F: Fn(&str, &str, &str) -> String,
{
    let mut b = Vec::new();

    {
        let reader = ParserConfig::new()
            .add_entity("nbsp", " ")
            .add_entity("copy", "©")
            .add_entity("reg", "®")
            .create_reader(xmldoc);
        let mut writer = EmitterConfig::default()
            .perform_indent(true)
            .create_writer(&mut b);

        for e in reader {
            match e {
                ev @ Ok(ReaderEvent::StartElement { .. }) => {
                    let ev = ev.unwrap();
                    let mut attrs: Vec<xml::attribute::OwnedAttribute> = vec![];

                    if let Some(WriterEvent::StartElement {
                        name,
                        attributes,
                        namespace,
                    }) = ev.as_writer_event()
                    {
                        for i in 0..attributes.len() {
                            let mut attr = attributes[i].to_owned();
                            let repl =
                                closure(&name.local_name, &attr.name.local_name, &attr.value);
                            attr.value = repl;
                            attrs.push(attr);
                        }

                        let w = WriterEvent::StartElement {
                            name,
                            attributes: Cow::Owned(attrs.iter().map(|x| x.borrow()).collect()),
                            //attributes: attributes,
                            namespace,
                        };
                        writer.write(w)?;
                    }
                }
                Ok(ReaderEvent::EndElement { name: n }) => {
                    if n.local_name.to_lowercase() == "head" && !extra_css.is_empty() {
                        // injecting here the extra css
                        let mut allcss = extra_css.concat();
                        allcss = String::from("*/") + &allcss + "/*";

                        writer.write(WriterEvent::start_element("style"))?;
                        writer.write("/*")?;
                        writer.write(WriterEvent::cdata(&allcss))?;
                        writer.write("*/")?;
                        writer.write(WriterEvent::end_element())?;
                    }
                    writer.write(WriterEvent::end_element())?;
                }
                ev @ Ok(_) => {
                    if let Some(e) = ev?.as_writer_event() {
                        writer.write(e)?;
                    }
                }
                Err(err) => {
                    return Err(XMLError {
                        error: String::from(err.msg()),
                    })
                }
            }
        }
    }

    Ok(b)
}
