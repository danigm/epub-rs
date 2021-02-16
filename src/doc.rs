//! Manages the epub doc.
//!
//! Provides easy methods to navigate througth the epub content, cover,
//! chapters, etc.

use anyhow::{anyhow, Error};
use xmlutils::XMLError;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::{Read, Seek};
use std::path::{Component, Path, PathBuf};

use crate::archive::EpubArchive;

use crate::xmlutils;

/// Struct that represent a navigation point in a table of content
#[derive(Eq)]
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
    fn cmp(&self, other: &NavPoint) -> Ordering {
        self.play_order.cmp(&other.play_order)
    }
}

impl PartialOrd for NavPoint {
    fn partial_cmp(&self, other: &NavPoint) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for NavPoint {
    fn eq(&self, other: &NavPoint) -> bool {
        self.play_order == other.play_order
    }
}

/// Struct to control the epub document
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
    /// #Examples
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
    pub fn new<P: AsRef<Path>>(path: P) -> Result<EpubDoc<BufReader<File>>, Error> {
        let path = path.as_ref();
        let file = File::open(path)?;
        let mut doc = EpubDoc::from_reader(BufReader::new(file))?;
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
    pub fn from_reader(reader: R) -> Result<EpubDoc<R>, Error> {
        let mut archive = EpubArchive::<R>::from_reader(reader)?;
        let spine: Vec<String> = vec![];
        let resources = HashMap::new();

        let container = archive.get_container_file()?;
        let root_file = get_root_file(container)?;
        let base_path = root_file.parent().expect("All files have a parent");
        let mut doc = EpubDoc {
            archive,
            spine,
            toc: vec![],
            resources,
            metadata: HashMap::new(),
            root_file: root_file.clone(),
            root_base: base_path.to_path_buf(),
            current: 0,
            extra_css: vec![],
            unique_identifier: None,
        };
        doc.fill_resources()?;
        Ok(doc)
    }

    /// Returns the first metadata found with this name.
    ///
    /// #Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let doc = doc.unwrap();
    /// let title = doc.mdata("title");
    /// assert_eq!(title.unwrap(), "Todo es mío");
    pub fn mdata(&self, name: &str) -> Option<String> {
        match self.metadata.get(name) {
            Some(v) => v.get(0).cloned(),
            None => None,
        }
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
    pub fn get_cover_id(&self) -> Result<String, Error> {
        match self.mdata("cover") {
            Some(id) => Ok(id),
            None => Err(anyhow!("Cover not found")),
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
    pub fn get_cover(&mut self) -> Result<Vec<u8>, Error> {
        let cover_id = self.get_cover_id()?;
        let cover_data = self.get_resource(&cover_id)?;
        Ok(cover_data)
    }

    /// Returns Release Identifier defined at
    /// https://www.w3.org/publishing/epub3/epub-packages.html#sec-metadata-elem-identifiers-pid
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
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_by_path<P: AsRef<Path>>(&mut self, path: P) -> Result<Vec<u8>, Error> {
        let content = self.archive.get_entry(path)?;
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine
    ///
    /// # Errors
    ///
    /// Returns an error if the id doesn't exists in the epub
    pub fn get_resource(&mut self, id: &str) -> Result<Vec<u8>, Error> {
        let path = match self.resources.get(id) {
            Some(s) => s.0.clone(),
            None => return Err(anyhow!("id not found")),
        };
        let content = self.get_resource_by_path(&path)?;
        Ok(content)
    }

    /// Returns the resource content by full path in the epub archive, as String
    ///
    /// # Errors
    ///
    /// Returns an error if the path doesn't exists in the epub
    pub fn get_resource_str_by_path<P: AsRef<Path>>(&mut self, path: P) -> Result<String, Error> {
        let content = self.archive.get_entry_as_str(path)?;
        Ok(content)
    }

    /// Returns the resource content by the id defined in the spine, as String
    ///
    /// # Errors
    ///
    /// Returns an error if the id doesn't exists in the epub
    pub fn get_resource_str(&mut self, id: &str) -> Result<String, Error> {
        let path = match self.resources.get(id) {
            Some(s) => s.0.clone(),
            None => return Err(anyhow!("id not found")),
        };
        let content = self.get_resource_str_by_path(path)?;
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
    pub fn get_resource_mime(&self, id: &str) -> Result<String, Error> {
        if let Some(&(_, ref res)) = self.resources.get(id) {
            return Ok(res.to_string());
        }
        Err(anyhow!("id not found"))
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
    /// # Errors
    ///
    /// Fails if the resource can't be found.
    pub fn get_resource_mime_by_path<P: AsRef<Path>>(&self, path: P) -> Result<String, Error> {
        let path = path.as_ref();

        for (_, v) in self.resources.iter() {
            if v.0 == path {
                return Ok(v.1.to_string());
            }
        }
        Err(anyhow!("path not found"))
    }

    /// Returns the current chapter content
    ///
    /// The current follows the epub spine order. You can modify the current
    /// calling to `go_next`, `go_prev` or `set_current` methods.
    ///
    /// # Errors
    ///
    /// This call shouldn't fail, but can return an error if the epub doc is
    /// broken.
    pub fn get_current(&mut self) -> Result<Vec<u8>, Error> {
        let current_id = self.get_current_id()?;
        self.get_resource(&current_id)
    }

    pub fn get_current_str(&mut self) -> Result<String, Error> {
        let current_id = self.get_current_id()?;
        self.get_resource_str(&current_id)
    }

    /// Returns the current chapter data, with resource uris renamed so they
    /// have the epub:// prefix and all are relative to the root file
    ///
    /// This method is useful to render the content with a html engine, because inside the epub
    /// local paths are relatives, so you can provide that content, because the engine will look
    /// for the relative path in the filesystem and that file isn't there. You should provide files
    /// with epub:// using the get_resource_by_path
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
    pub fn get_current_with_epub_uris(&mut self) -> Result<Vec<u8>, Error> {
        let path = self.get_current_path()?;
        let current = self.get_current()?;

        let resp = xmlutils::replace_attrs(
            current.as_slice(),
            |element, attr, value| match (element, attr) {
                ("link", "href") => build_epub_uri(&path, value),
                ("img", "src") => build_epub_uri(&path, value),
                ("image", "href") => build_epub_uri(&path, value),
                ("a", "href") => build_epub_uri(&path, value),
                _ => String::from(value),
            },
            &self.extra_css,
        );

        match resp {
            Ok(a) => Ok(a),
            Err(error) => Err(anyhow!("{}", error.error)),
        }
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
    pub fn get_current_mime(&self) -> Result<String, Error> {
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
    pub fn get_current_path(&self) -> Result<PathBuf, Error> {
        let current_id = self.get_current_id()?;
        match self.resources.get(&current_id) {
            Some(&(ref p, _)) => Ok(p.clone()),
            None => Err(anyhow!("Current not found")),
        }
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
    pub fn get_current_id(&self) -> Result<String, Error> {
        let current_id = self.spine.get(self.current);
        match current_id {
            Some(id) => Ok(id.to_string()),
            None => Err(anyhow!("current is broken")),
        }
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
    /// assert!(doc.go_next().is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// If the page is the last, will not change and an error will be returned
    pub fn go_next(&mut self) -> Result<(), Error> {
        if self.current + 1 >= self.spine.len() {
            return Err(anyhow!("last page"));
        }
        self.current += 1;
        Ok(())
    }

    /// Changes current to the prev chapter
    ///
    /// # Examples
    ///
    /// ```
    /// # use epub::doc::EpubDoc;
    /// # let doc = EpubDoc::new("test.epub");
    /// # let mut doc = doc.unwrap();
    /// assert!(doc.go_prev().is_err());
    ///
    /// doc.go_next(); // 000.xhtml
    /// doc.go_next(); // 001.xhtml
    /// doc.go_next(); // 002.xhtml
    /// doc.go_prev(); // 001.xhtml
    /// assert_eq!("001.xhtml", doc.get_current_id().unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// If the page is the first, will not change and an error will be returned
    pub fn go_prev(&mut self) -> Result<(), Error> {
        if self.current < 1 {
            return Err(anyhow!("first page"));
        }
        self.current -= 1;
        Ok(())
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
    /// assert!(doc.set_current_page(50).is_err());
    /// ```
    ///
    /// # Errors
    ///
    /// If the page isn't valid, will not change and an error will be returned
    pub fn set_current_page(&mut self, n: usize) -> Result<(), Error> {
        if n >= self.spine.len() {
            return Err(anyhow!("page not valid"));
        }
        self.current = n;
        Ok(())
    }

    /// This will inject this css in every html page getted with the
    /// get_current_with_epub_uris call
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
    /// This method is useful to convert a toc NavPoint content to a chapter number
    /// to be able to navigate easily
    pub fn resource_uri_to_chapter(&self, uri: &PathBuf) -> Option<usize> {
        for (k, (path, _mime)) in self.resources.iter() {
            if path == uri {
                return self.resource_id_to_chapter(&k);
            }
        }

        None
    }

    /// Function to convert a resource id to a chapter number in the spine
    /// If the resourse isn't in the spine list, None will be returned
    pub fn resource_id_to_chapter(&self, uri: &str) -> Option<usize> {
        self.spine.iter().position(|item| item == uri)
    }

    // Forcibly converts separators in a filepath to unix separators to
    // to ensure that ZipArchive's by_name method will retrieve the proper
    // file. Failing to convert to unix-style on Windows causes the
    // ZipArchive not to find the file.
    fn convert_path_separators(&self, href: &str) -> PathBuf {
        let path = self.root_base.join(href.split("/").collect::<PathBuf>());
        if cfg!(windows) {
            let path = path.as_path().display().to_string().replace("\\", "/");
            return PathBuf::from(path);
        }
        PathBuf::from(path)
    }

    fn fill_resources(&mut self) -> Result<(), Error> {
        let container = self.archive.get_entry(&self.root_file)?;
        let root = xmlutils::XMLReader::parse(container.as_slice())?;
        let unique_identifier_id = &root.borrow().get_attr("unique-identifier").ok();
        // resources from manifest
        let manifest = root.borrow().find("manifest")?;
        for r in manifest.borrow().childs.iter() {
            let item = r.borrow();
            let _ = self.insert_resource(&item);
        }
        // items from spine
        let spine = root.borrow().find("spine")?;
        for r in spine.borrow().childs.iter() {
            let item = r.borrow();
            let _ = self.insert_spine(&item);
        }
        // toc.ncx
        if let Ok(toc) = spine.borrow().get_attr("toc") {
            let _ = self.fill_toc(&toc);
        }
        // metadata
        let metadata = root.borrow().find("metadata")?;
        for r in metadata.borrow().childs.iter() {
            let item = r.borrow();
            if item.name.local_name == "meta" {
                if let (Ok(k), Ok(v)) = (item.get_attr("name"), item.get_attr("content")) {
                    self.metadata.entry(k).or_insert(vec![]).push(v);
                } else if let Ok(k) = item.get_attr("property") {
                    let v = match item.text {
                        Some(ref x) => x.to_string(),
                        None => String::from(""),
                    };
                    self.metadata.entry(k).or_insert(vec![]).push(v);
                }
            } else {
                let k = &item.name.local_name;
                let v = match item.text {
                    Some(ref x) => x.to_string(),
                    None => String::from(""),
                };
                if k == "identifier"
                    && self.unique_identifier.is_none()
                    && unique_identifier_id.is_some()
                {
                    if let Ok(id) = item.get_attr("id") {
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
                    self.metadata.insert(k.to_string(), vec![v]);
                }
            }
        }
        Ok(())
    }

    fn insert_resource(&mut self, item: &xmlutils::XMLNode) -> Result<(), XMLError> {
        let id = item.get_attr("id")?;
        let href = item.get_attr("href")?;
        let mtype = item.get_attr("media-type")?;
        let path = self.convert_path_separators(&href);
        self.resources
            .insert(id, (path, mtype));
        Ok(())
    }

    fn insert_spine(&mut self, item:&xmlutils::XMLNode) -> Result<(), XMLError> {
        let id = item.get_attr("idref")?;
        self.spine.push(id);
        Ok(())
    }

    fn fill_toc(&mut self, id: &str) -> Result<(), Error> {
        let toc_res = self
            .resources
            .get(id)
            .ok_or_else(|| anyhow!("No toc found"))?;

        let container = self.archive.get_entry(&toc_res.0)?;
        let root = xmlutils::XMLReader::parse(container.as_slice())?;

        let mapnode = root.borrow().find("navMap")?;

        self.toc.append(&mut self.get_navpoints(&mapnode.borrow()));
        self.toc.sort();

        Ok(())
    }

    /// Recursively extract all navpoints from a node.
    fn get_navpoints(&self, parent: &xmlutils::XMLNode) -> Vec<NavPoint> {
        let mut navpoints = Vec::new();

        // TODO: get docTitle
        // TODO: parse metadata (dtb:totalPageCount, dtb:depth, dtb:maxPageNumber)

        for nav in parent.childs.iter() {
            let item = nav.borrow();
            if item.name.local_name != "navPoint" {
                continue;
            }
            let play_order = item
                .get_attr("playOrder")
                .ok()
                .and_then(|n| usize::from_str_radix(&n, 10).ok());
            let content = match item.find("content") {
                Ok(c) => c
                    .borrow()
                    .get_attr("src")
                    .ok()
                    .map(|p| self.root_base.join(p)),
                _ => None,
            };
            let label = match item.find("navLabel") {
                Ok(l) => l
                    .borrow()
                    .childs
                    .get(0)
                    .and_then(|t| t.borrow().text.clone()),
                _ => None,
            };

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

fn get_root_file(container: Vec<u8>) -> Result<PathBuf, Error> {
    let root = xmlutils::XMLReader::parse(container.as_slice())?;
    let el = root.borrow();
    let element = el.find("rootfile")?;
    let el2 = element.borrow();

    let attr = el2.get_attr("full-path")?;

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
        cpath.display().to_string().replace("\\", "/")
    } else {
        cpath.display().to_string()
    };

    format!("epub://{}", path)
}
