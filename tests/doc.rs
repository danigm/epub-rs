use epub::doc::EpubDoc;
use std::path::Path;

#[test]
fn doc_open() {
    let doc = EpubDoc::new("test.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    let doc2 = EpubDoc::new("tests/docs/Metamorphosis-jackson.epub").unwrap();
    assert_eq!(Path::new("OEBPS"), doc.root_base);
    assert_eq!(Path::new("OEBPS/content.opf"), doc.root_file);

    assert_eq!(23, doc.resources.len());
    {
        let tpage = doc.resources.get("titlepage.xhtml");
        assert_eq!(tpage.unwrap().0, Path::new("OEBPS/Text/titlepage.xhtml"));
    }

    {
        assert_eq!(17, doc.spine.len());
        assert_eq!("titlepage.xhtml", doc.spine[0]);
    }

    {
        assert_eq!(None, doc.page_progression_direction);
    }

    {
        let unique_identifier = doc.unique_identifier.clone();
        assert_eq!(
            unique_identifier.unwrap(),
            "urn:uuid:09132750-3601-4d19-b3a4-55fdf8639849"
        );
    }

    {
        let title = doc.mdata("title");
        assert_eq!(title.unwrap(), "Todo es mío");
    }

    {
        let cover = doc.get_cover_id();
        assert_eq!(cover, Some("portada.png".into()));
    }

    {
        let modified = doc.mdata("dcterms:modified");
        assert_eq!(modified.unwrap(), "2015-08-10T18:12:03Z");
    }

    {
        let release_identifier = doc.get_release_identifier();
        assert_eq!(
            release_identifier.unwrap(),
            "urn:uuid:09132750-3601-4d19-b3a4-55fdf8639849@2015-08-10T18:12:03Z"
        );
    }

    {
        let unique_identifier = doc2.unique_identifier.clone();
        assert_eq!(
            "http://metamorphosiskafka.pressbooks.com",
            unique_identifier.unwrap()
        );
    }

    {
        let release_identifier = doc2.get_release_identifier();
        assert_eq!(None, release_identifier);
    }
}

#[test]
fn toc_test() {
    let doc = EpubDoc::new("test.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();

    assert!(!doc.toc.is_empty());
    for nav in doc.toc.iter() {
        let chapter = doc.resource_uri_to_chapter(&nav.content);
        assert!(chapter.is_some());
        assert_eq!(nav.play_order, chapter.unwrap());
    }
}
