extern crate epub;

use epub::doc::EpubDoc;

#[test]
fn doc_open() {
    let doc = EpubDoc::new("test.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    assert_eq!("OEBPS/", doc.root_base);
    assert_eq!("OEBPS/content.opf", doc.root_file);
    assert_eq!(21, doc.resources.len());
    assert_eq!(17, doc.spine.len());
    assert_eq!("titlepage.xhtml", doc.spine[0]);
}
