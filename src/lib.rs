#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::module_name_repetitions,
    clippy::let_underscore_drop,

    // for MSRV
    clippy::unnested_or_patterns,
    clippy::uninlined_format_args,
    clippy::missing_const_for_fn,
)]

//! EPUB library
//! lib to read and navigate through an epub file contents
//!
//! # Examples
//!
//! ## Opening
//!
//! ```
//! use epub::doc::EpubDoc;
//! let doc = EpubDoc::new("test.epub");
//! assert!(doc.is_ok());
//! let doc = doc.unwrap();
//!
//! ```
//!
//! ## Getting doc metatada
//!
//! Metadata is a [`HashMap`](std::collections::HashMap) storing all metadata defined in the epub
//!
//! ```
//! # use epub::doc::EpubDoc;
//! # let doc = EpubDoc::new("test.epub");
//! # let doc = doc.unwrap();
//! let title = doc.mdata("title");
//! assert_eq!(title.unwrap(), "Todo es mío");
//! ```
//!
//! ## Accessing resources
//!
//! In the resources var is stored each resource defined
//! in the epub indexed by the id and with the full internal
//! path and mimetype. It's a `HashMap<a: String, (b: String, c: String)>`
//! where `a` is the resource id, `b` is the resource full path and
//! `c` is the resource mimetype
//!
//! ```
//! # use epub::doc::EpubDoc;
//! # use std::path::Path;
//! # let doc = EpubDoc::new("test.epub");
//! # let doc = doc.unwrap();
//! assert_eq!(23, doc.resources.len());
//! let tpage = doc.resources.get("titlepage.xhtml");
//! assert_eq!(tpage.unwrap().0, Path::new("OEBPS/Text/titlepage.xhtml"));
//! assert_eq!(tpage.unwrap().1, "application/xhtml+xml");
//! ```
//!
//! ## Navigating using the spine
//!
//! Spine is a `Vec<String>` storing the epub spine as resources ids
//!
//! ```
//! # use epub::doc::EpubDoc;
//! # let doc = EpubDoc::new("test.epub");
//! # let doc = doc.unwrap();
//! assert_eq!(17, doc.spine.len());
//! assert_eq!("titlepage.xhtml", doc.spine[0]);
//! ```
//!
//! ## Navigation using the doc internal state
//!
//! ```
//! use epub::doc::EpubDoc;
//! let doc = EpubDoc::new("test.epub");
//! let mut doc = doc.unwrap();
//! assert_eq!(0, doc.get_current_page());
//! assert_eq!("application/xhtml+xml", doc.get_current_mime().unwrap());
//!
//! doc.go_next();
//! assert_eq!("000.xhtml", doc.get_current_id().unwrap());
//! doc.go_next();
//! assert_eq!("001.xhtml", doc.get_current_id().unwrap());
//! doc.go_prev();
//! assert_eq!("000.xhtml", doc.get_current_id().unwrap());
//!
//! doc.set_current_page(2);
//! assert_eq!("001.xhtml", doc.get_current_id().unwrap());
//! assert_eq!(2, doc.get_current_page());
//! assert!(!doc.set_current_page(50));
//!
//! // doc.get_current() will return a Vec<u8> with the current page content
//! // doc.get_current_str() will return a String with the current page content
//! ```
//!
//! ## Getting the cover
//!
//! ```ignore
//! use std::fs;
//! use std::io::Write;
//! use epub::doc::EpubDoc;
//!
//! let doc = EpubDoc::new("test.epub");
//! assert!(doc.is_ok());
//! let mut doc = doc.unwrap();
//!
//! let cover_data = doc.get_cover().unwrap();
//!
//! let f = fs::File::create("/tmp/cover.png");
//! assert!(f.is_ok());
//! let mut f = f.unwrap();
//! let resp = f.write_all(&cover_data);
//! ```

mod xmlutils;

pub mod archive;
pub mod doc;
