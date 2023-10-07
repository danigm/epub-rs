//! Manages the epub doc.
//!
//! Provides easy methods to navigate through the epub content, cover,
//! chapters, etc.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};
use xmlutils::XMLError;

use crate::archive::EpubArchive;

use crate::xmlutils;

#[derive(Debug, thiserror::Error)]
pub enum DocError {
    #[error("Archive Error: {0}")]
    ArchiveError(#[from] crate::archive::ArchiveError),
    #[error("XML Error: {0}")]
    XmlError(#[from] crate::xmlutils::XMLError),
    #[error("I/O Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Invalid EPub")]
    InvalidEpub,
}

/// Struct that represent a navigation point in a table of content
#[derive(Clone, Debug, Eq)]
pub struct NavPoint {
    /// the title of this navpoint
    pub label: String,
    /// the resource path
    pub content: PathBuf,
    /// nested navpoints
    pub children: Vec<NavPoint>,
    /// the order in the toc
    pub play_order: usize,
}

impl Ord for NavPoint {
    fn cmp(&self, other: &Self) -> Ordering {
        self.play_order.cmp(&other.play_order)
    }
}

impl PartialOrd for NavPoint {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NavPoint {
    fn eq(&self, other: &Self) -> bool {
        self.play_order == other.play_order
    }
}

/// Struct to control the epub document
#[derive(Clone, Debug)]
pub struct EpubDoc<R: Read + Seek> {
    /// the zip archive
    archive: EpubArchive<R>,

    /// The current chapter, is an spine index
    current: usize,

    /// epub spine ids
    pub spine: Vec<String>,

    /// resource id -> (path, mime)
    pub resources: HashMap<String, (PathBuf, String)>,

    /// table of content, list of `NavPoint` in the toc.ncx
    pub toc: Vec<NavPoint>,

    /// The epub metadata stored as key -> value
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.metadata.get("title");
    /// assert_eq!(title.unwrap(), &vec!["Todo es mío".to_string()]);
    /// ```
    pub metadata: HashMap<String, Vec<String>>,

    /// root file base path
    pub root_base: PathBuf,

    /// root file full path
    pub root_file: PathBuf,

    /// Custom css list to inject in every xhtml file
    pub extra_css: Vec<String>,

    /// unique identifier
    pub unique_identifier: Option<String>,

    /// The id of the cover, if any
    pub cover_id: Option<String>,

    /// The global direction in which the content flows
    pub page_progression_direction: Option<String>,
}

impl EpubDoc<BufReader<File>> {
    /// Opens the epub file in `path`.
    ///
    /// Initialize some internal variables to be able to access to the epub
    /// spine definition and to navigate trhough the epub.
    ///
    /// # Examples
    ///
    /// ```
    /// use epub::doc::EpubDoc;
    ///
    /// let doc = EpubDoc::new("test.epub");
    /// assert!(doc.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the epub is broken or if the file doesn't
    /// exists.
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, DocError> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut doc = Self::from_reader(BufReader::new(file))?;
        doc.archive.path = path.to_path_buf();
        Ok(doc)
    }
}

impl<R: Read + Seek> EpubDoc<R> {
    /// Opens the epub contained in `reader`.
    ///
    /// Initialize some internal variables to be able to access to the epub
    /// spine definition and to navigate trhough the epub.
    ///
    /// # Examples
    ///
    /// ```
    /// use epub::doc::EpubDoc;
    /// use std::fs::File;
    /// use std::io::{Cursor, Read};
    ///
    /// let mut file = File::open("test.epub").unwrap();
    /// let mut buffer = Vec::new();
    /// file.read_to_end(&mut buffer).unwrap();
    ///
    /// let cursor = Cursor::new(buffer);
    ///
    /// let doc = EpubDoc::from_reader(cursor);
    /// assert!(doc.is_ok());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the epub is broken.
    pub fn from_reader(reader: R) -> Result<Self, DocError> {
        let mut archive = EpubArchive::from_reader(reader)?;

        let container = archive.get_container_file()?;
        let root_file = get_root_file(&container)?;
        let base_path = root_file.parent().expect("All files have a parent");
        let mut doc = Self {
            archive,
            spine: vec![],
            toc: vec![],
            resources: HashMap::new(),
            metadata: HashMap::new(),
            root_file: root_file.clone(),
            root_base: base_path.to_path_buf(),
            current: 0,
            extra_css: vec![],
            unique_identifier: None,
            cover_id: None,
            page_progression_direction: None,
        };
        doc.fill_resources()?;
        Ok(doc)
    }

    /// Returns the first metadata found with this name.
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.mdata("title");
    /// assert_eq!(title.unwrap(), "Todo es mío");
    pub fn mdata(&self, name: &str) -> Option<String> {
        self.metadata.get(name).and_then(|v| v.get(0).cloned())
    }

    /// Returns the id of the epub cover.
    ///
    /// The cover is searched in the doc metadata, by the tag `<meta name="cover" value"..">`
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
    /// let cover_id = doc.get_cover_id();
    /// ```
    ///
    /// This returns the cover id, which can be used to get the cover data.
    /// The id is not guaranteed to be valid.
    pub fn get_cover_id(&self) -> Option<String> {
        self.cover_id.clone()
    }

    /// Returns the cover's content and mime-type
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
    /// Returns [`None`] if the cover can't be found.
    pub fn get_cover(&mut self) -> Option<(Vec<u8>, String)> {
        let cover_id = self.get_cover_id();
        cover_id.and_then(|cid| self.get_resource(&cid))
    }

    /// Returns Release Identifier defined at
    /// <https://www.w3.org/publishing/epub3/epub-packages.html#sec-metadata-elem-identifiers-pid>
    pub fn get_release_identifier(&self) -> Option<String> {
        match (
            self.unique_identifier.as_ref(),
            self.mdata("dcterms:modified"),
        ) {
            (Some(unique_identifier), Some(modified)) => {
                Some(format!("{}@{}", unique_identifier, modified))
            }
            _ => None,
        }
    }

    /// Returns the resource content by full path in the epub archive
    ///
    /// Returns [`None`] if the path doesn't exist in the epub
    pub fn get_resource_by_path<P: AsRef<Path>>(&mut self, path: P) -> Option<Vec<u8>> {
        self.archive.get_entry(path).ok()
    }

    /// Returns the resource content and mime-type by the id defined in the spine
    ///
    /// Returns [`None`] if the id doesn't exists in the epub
    pub fn get_resource(&mut self, id: &str) -> Option<(Vec<u8>, String)> {
        let (path, mime) = self.resources.get(id)?;
        let path = path.clone();
        let mime = mime.clone();
        let content = self.get_resource_by_path(&path)?;
        Some((content, mime))
    }

    /// Returns the resource content by full path in the epub archive, as String
    ///
    /// Returns [`None`] if the path doesn't exists in the epub
    pub fn get_resource_str_by_path<P: AsRef<Path>>(&mut self, path: P) -> Option<String> {
        self.archive.get_entry_as_str(path).ok()
    }

    /// Returns the resource content and mime-type by the id defined in the spine, as String
    ///
    /// Returns [`None`] if the id doesn't exists in the epub
    pub fn get_resource_str(&mut self, id: &str) -> Option<(String, String)> {
        let (path, mime) = self.resources.get(id)?;
        let mime = mime.clone();
        let path = path.clone();
        let content = self.get_resource_str_by_path(path)?;
        Some((content, mime))
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
    ///
    /// Returns [`None`] the resource can't be found.
    pub fn get_resource_mime(&self, id: &str) -> Option<String> {
        self.resources.get(id).map(|r| r.1.clone())
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
    ///
    /// Returns [`None`] the resource can't be found.
    pub fn get_resource_mime_by_path<P: AsRef<Path>>(&self, path: P) -> Option<String> {
        let path = path.as_ref();

        self.resources.iter().find_map(|(_, r)| {
            if r.0 == path {
                Some(r.1.to_string())
            } else {
                None
            }
        })
    }

    /// Returns the current chapter content and mime-type
    ///
    /// The current follows the epub spine order. You can modify the current
    /// calling to `go_next`, `go_prev` or `set_current` methods.
    ///
    /// Can return [`None`] if the epub is broken.
    pub fn get_current(&mut self) -> Option<(Vec<u8>, String)> {
        let current_id = self.get_current_id()?;
        self.get_resource(&current_id)
    }

    /// See [`Self::get_current`]
    pub fn get_current_str(&mut self) -> Option<(String, String)> {
        let current_id = self.get_current_id()?;
        self.get_resource_str(&current_id)
    }

    /// Returns the current chapter data, with resource uris renamed so they
    /// have the epub:// prefix and all are relative to the root file
    ///
    /// This method is useful to render the content with a html engine, because inside the epub
    /// local paths are relatives, so you can provide that content, because the engine will look
    /// for the relative path in the filesystem and that file isn't there. You should provide files
    /// with epub:// using [`Self::get_resource_by_path`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let mut doc = EpubDoc::new("test.epub").unwrap();
    /// let current = doc.get_current_with_epub_uris().unwrap();
    /// let text = String::from_utf8(current).unwrap();
    /// assert!(text.contains("epub://OEBPS/Images/portada.png"));

    /// doc.go_next();
    /// let current = doc.get_current_with_epub_uris().unwrap();
    /// let text = String::from_utf8(current).unwrap();
    /// assert!(text.contains("epub://OEBPS/Styles/stylesheet.css"));
    /// assert!(text.contains("http://creativecommons.org/licenses/by-sa/3.0/"));
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`DocError::InvalidEpub`] if the epub is broken.
    pub fn get_current_with_epub_uris(&mut self) -> Result<Vec<u8>, DocError> {
        let path = self.get_current_path().ok_or(DocError::InvalidEpub)?;
        let (current, _mime) = self.get_current().ok_or(DocError::InvalidEpub)?;

        let resp = xmlutils::replace_attrs(
            current.as_slice(),
            |element, attr, value| match (element, attr) {
                ("link", "href") | ("image", "href") | ("a", "href") | ("img", "src") => {
                    build_epub_uri(&path, value)
                }
                _ => String::from(value),
            },
            &self.extra_css,
        );

        resp.map_err(From::from)
    }

    /// Returns the current chapter mimetype
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let m = doc.get_current_mime();
    /// assert_eq!("application/xhtml+xml", m.unwrap());
    /// ```
    ///
    /// Can return [`None`] if the epub is broken.
    pub fn get_current_mime(&self) -> Option<String> {
        let current_id = self.get_current_id()?;
        self.get_resource_mime(&current_id)
    }

    /// Returns the current chapter full path
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # use std::path::Path;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let p = doc.get_current_path();
    /// assert_eq!(Path::new("OEBPS/Text/titlepage.xhtml"), p.unwrap());
    /// ```
    ///
    /// Can return [`None`] if the epub is broken.
    pub fn get_current_path(&self) -> Option<PathBuf> {
        let current_id = self.get_current_id()?;
        self.resources.get(&current_id).map(|r| r.0.clone())
    }

    /// Returns the current chapter id
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let id = doc.get_current_id();
    /// assert_eq!("titlepage.xhtml", id.unwrap());
    /// ```
    ///
    /// Can return [`None`] if the epub is broken.
    pub fn get_current_id(&self) -> Option<String> {
        self.spine.get(self.current).cloned()
    }

    /// Changes current to the next chapter
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// doc.go_next();
    /// assert_eq!("000.xhtml", doc.get_current_id().unwrap());
    ///
    /// let len = doc.spine.len();
    /// for i in 1..len {
    ///     doc.go_next();
    /// }
    /// assert!(!doc.go_next());
    /// ```
    ///
    /// Returns [`false`] if the current chapter is the last one
    pub fn go_next(&mut self) -> bool {
        if self.current + 1 >= self.spine.len() {
            false
        } else {
            self.current += 1;
            true
        }
    }

    /// Changes current to the prev chapter
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert!(!doc.go_prev());
    ///
    /// doc.go_next(); // 000.xhtml
    /// doc.go_next(); // 001.xhtml
    /// doc.go_next(); // 002.xhtml
    /// doc.go_prev(); // 001.xhtml
    /// assert_eq!("001.xhtml", doc.get_current_id().unwrap());
    /// ```
    ///
    /// Returns [`false`] if the current chapter is the first one
    pub fn go_prev(&mut self) -> bool {
        if self.current < 1 {
            false
        } else {
            self.current -= 1;
            true
        }
    }

    /// Returns the number of chapters
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert_eq!(17, doc.get_num_pages());
    /// ```
    pub fn get_num_pages(&self) -> usize {
        self.spine.len()
    }

    /// Returns the current chapter number, starting from 0
    pub fn get_current_page(&self) -> usize {
        self.current
    }

    /// Changes the current page
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert_eq!(0, doc.get_current_page());
    /// doc.set_current_page(2);
    /// assert_eq!("001.xhtml", doc.get_current_id().unwrap());
    /// assert_eq!(2, doc.get_current_page());
    /// assert!(!doc.set_current_page(50));
    /// ```
    ///
    /// Returns [`false`] if the page is out of bounds
    pub fn set_current_page(&mut self, n: usize) -> bool {
        if n >= self.spine.len() {
            false
        } else {
            self.current = n;
            true
        }
    }

    /// This will inject this css in every html page getted with
    /// [`Self::get_current_with_epub_uris`]
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// # let _ = doc.set_current_page(2);
    /// let extracss = "body { background-color: black; color: white }";
    /// doc.add_extra_css(extracss);
    /// let current = doc.get_current_with_epub_uris().unwrap();
    /// let text = String::from_utf8(current).unwrap();
    /// assert!(text.contains(extracss));
    /// ```
    pub fn add_extra_css(&mut self, css: &str) {
        self.extra_css.push(String::from(css));
    }

    /// Function to convert a resource path to a chapter number in the spine
    /// If the resourse isn't in the spine list, None will be returned
    ///
    /// This method is useful to convert a toc [`NavPoint`] content to a chapter number
    /// to be able to navigate easily
    pub fn resource_uri_to_chapter(&self, uri: &PathBuf) -> Option<usize> {
        for (k, (path, _mime)) in &self.resources {
            if path == uri {
                return self.resource_id_to_chapter(k);
            }
        }

        None
    }

    /// Function to convert a resource id to a chapter number in the spine
    /// If the resourse isn't in the spine list, None will be returned
    pub fn resource_id_to_chapter(&self, uri: &str) -> Option<usize> {
        self.spine.iter().position(|item| item == uri)
    }

    fn fill_resources(&mut self) -> Result<(), DocError> {
        let container = self.archive.get_entry(&self.root_file)?;
        let root = xmlutils::XMLReader::parse(container.as_slice())?;
        let unique_identifier_id = &root.borrow().get_attr("unique-identifier");

        // resources from manifest
        // This should be run before everything else, because other functions relies on
        // self.resources and should be filled before calling `fill_toc`
        let manifest = root
            .borrow()
            .find("manifest")
            .ok_or(DocError::InvalidEpub)?;
        for r in &manifest.borrow().children {
            let item = r.borrow();
            if self.cover_id.is_none() {
                if let (Some(id), Some(property)) =
                    (item.get_attr("id"), item.get_attr("properties"))
                {
                    if property == "cover-image" {
                        self.cover_id = Some(id);
                    }
                }
            }
            let _ = self.insert_resource(&item);
        }

        // items from spine
        let spine = root.borrow().find("spine").ok_or(DocError::InvalidEpub)?;
        for r in &spine.borrow().children {
            let item = r.borrow();
            let _ = self.insert_spine(&item);
        }

        // toc.ncx
        if let Some(toc) = spine.borrow().get_attr("toc") {
            let _ = self.fill_toc(&toc);
        }

        // page progression direction
        self.page_progression_direction = spine.borrow().get_attr("page-progression-direction");

        // metadata
        let metadata = root
            .borrow()
            .find("metadata")
            .ok_or(DocError::InvalidEpub)?;
        for r in &metadata.borrow().children {
            let item = r.borrow();
            if item.name.local_name == "meta" {
                if let (Some(k), Some(v)) = (item.get_attr("name"), item.get_attr("content")) {
                    if k == "cover" {
                        self.cover_id = Some(v.clone());
                    }
                    self.metadata.entry(k).or_default().push(v);
                } else if let Some(k) = item.get_attr("property") {
                    let v = item.text.clone().unwrap_or_default();
                    self.metadata.entry(k).or_default().push(v);
                }
            } else {
                let k = &item.name.local_name;
                let v = item.text.clone().unwrap_or_default();
                if k == "identifier"
                    && self.unique_identifier.is_none()
                    && unique_identifier_id.is_some()
                {
                    if let Some(id) = item.get_attr("id") {
                        if &id == unique_identifier_id.as_ref().unwrap() {
                            self.unique_identifier = Some(v.to_string());
                        }
                    }
                }
                if self.metadata.contains_key(k) {
                    if let Some(arr) = self.metadata.get_mut(k) {
                        arr.push(v);
                    }
                } else {
                    self.metadata.insert(k.clone(), vec![v]);
                }
            }
        }

        Ok(())
    }

    // Forcibly converts separators in a filepath to unix separators to
    // to ensure that ZipArchive's by_name method will retrieve the proper
    // file. Failing to convert to unix-style on Windows causes the
    // ZipArchive not to find the file.
    fn convert_path_seps<P: AsRef<Path>>(&self, href: P) -> PathBuf {
        let mut path = self.root_base.join(href);
        if cfg!(windows) {
            path = PathBuf::from(path.to_string_lossy().replace('\\', "/"));
        }
        path
    }

    fn insert_resource(&mut self, item: &xmlutils::XMLNode) -> Result<(), XMLError> {
        let id = item
            .get_attr("id")
            .ok_or_else(|| XMLError::AttrNotFound("id".into()))?;
        let href = item
            .get_attr("href")
            .ok_or_else(|| XMLError::AttrNotFound("href".into()))?;
        let mtype = item
            .get_attr("media-type")
            .ok_or_else(|| XMLError::AttrNotFound("media-type".into()))?;

        self.resources
            .insert(id, (self.convert_path_seps(href), mtype));
        Ok(())
    }

    fn insert_spine(&mut self, item: &xmlutils::XMLNode) -> Result<(), DocError> {
        let id = item
            .get_attr("idref")
            .ok_or_else(|| XMLError::AttrNotFound("idref".into()))?;
        self.spine.push(id);
        Ok(())
    }

    fn fill_toc(&mut self, id: &str) -> Result<(), DocError> {
        let toc_res = self.resources.get(id).ok_or(DocError::InvalidEpub)?; // this should be turned into it's own error type, but

        let container = self.archive.get_entry(&toc_res.0)?;
        let root = xmlutils::XMLReader::parse(container.as_slice())?;

        let mapnode = root
            .borrow()
            .find("navMap")
            .ok_or_else(|| XMLError::AttrNotFound("navMap".into()))?;

        self.toc.append(&mut self.get_navpoints(&mapnode.borrow()));
        self.toc.sort();

        Ok(())
    }

    /// Recursively extract all navpoints from a node.
    fn get_navpoints(&self, parent: &xmlutils::XMLNode) -> Vec<NavPoint> {
        let mut navpoints = Vec::new();

        // TODO: get docTitle
        // TODO: parse metadata (dtb:totalPageCount, dtb:depth, dtb:maxPageNumber)

        for nav in &parent.children {
            let item = nav.borrow();
            if item.name.local_name != "navPoint" {
                continue;
            }
            let play_order = item.get_attr("playOrder").and_then(|n| n.parse().ok());
            let content = item
                .find("content")
                .and_then(|c| c.borrow().get_attr("src").map(|p| self.root_base.join(p)));

            let label = item.find("navLabel").and_then(|l| {
                l.borrow()
                    .children
                    .get(0)
                    .and_then(|t| t.borrow().text.clone())
            });

            if let (Some(o), Some(c), Some(l)) = (play_order, content, label) {
                let navpoint = NavPoint {
                    label: l.clone(),
                    content: c.clone(),
                    children: self.get_navpoints(&item),
                    play_order: o,
                };
                navpoints.push(navpoint);
            }
        }

        navpoints.sort();
        navpoints
    }
}

fn get_root_file(container: &[u8]) -> Result<PathBuf, DocError> {
    let root = xmlutils::XMLReader::parse(container)?;
    let el = root.borrow();
    let element = el
        .find("rootfile")
        .ok_or_else(|| XMLError::AttrNotFound("rootfile".into()))?;
    let el2 = element.borrow();

    let attr = el2
        .get_attr("full-path")
        .ok_or_else(|| XMLError::AttrNotFound("full-path".into()))?;

    Ok(PathBuf::from(attr))
}

fn build_epub_uri<P: AsRef<Path>>(path: P, append: &str) -> String {
    // allowing external links
    if append.starts_with("http") {
        return String::from(append);
    }

    let path = path.as_ref();
    let mut cpath = path.to_path_buf();

    // current file base dir
    cpath.pop();
    for p in Path::new(append).components() {
        match p {
            Component::ParentDir => {
                cpath.pop();
            }
            Component::Normal(s) => {
                cpath.push(s);
            }
            _ => {}
        };
    }

    // If on Windows, replace all Windows path separators with Unix path separators
    let path = if cfg!(windows) {
        cpath.to_string_lossy().replace('\\', "/")
    } else {
        cpath.to_string_lossy().to_string()
    };

    format!("epub://{}", path)
}
