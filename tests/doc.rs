extern crate epub;

use std::fs;
use std::io::Write;

use epub::doc::EpubDoc;

#[test]
fn doc_open() {
    let doc = EpubDoc::new("test.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    assert_eq!("OEBPS/", doc.root_base);
    assert_eq!("OEBPS/content.opf", doc.root_file);

    assert_eq!(21, doc.resources.len());
    {
        let tpage = doc.resources.get("titlepage.xhtml");
        assert_eq!(tpage.unwrap().0, "OEBPS/Text/titlepage.xhtml");
    }

    {
        assert_eq!(17, doc.spine.len());
        assert_eq!("titlepage.xhtml", doc.spine[0]);
    }

    {
        let title = doc.metadata.get("title");
        assert_eq!(title.unwrap(), "Todo es mío");
    }

    {
        let cover = doc.get_cover();
        assert_eq!(cover.unwrap(), "portada.png");
    }
}
