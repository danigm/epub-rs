use epub::doc::EpubDoc;
use epub::doc::EpubVersion;
use epub::doc::MetadataItem;
use std::path::Path;

#[test]
#[cfg(feature = "mock")]
fn doc_mock() {
    let doc = EpubDoc::mock();
    assert!(doc.is_ok());
}

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
        assert_eq!("titlepage.xhtml", doc.spine[0].idref);
    }

    {
        let unique_identifier = doc.unique_identifier.clone();
        assert_eq!(
            unique_identifier.unwrap(),
            "urn:uuid:09132750-3601-4d19-b3a4-55fdf8639849"
        );
    }

    {
        let identifier = doc.mdata("identifier").unwrap();
        let scheme = identifier.refinement("scheme").unwrap();
        assert_eq!(scheme.value, "UUID");
    }

    {
        let title = doc.mdata("title");
        assert_eq!(title.unwrap().value, "Todo es mío");
    }

    {
        let creator = doc.mdata("creator").unwrap();
        assert_eq!(creator.value, "Daniel Garcia");
        let role = creator.refinement("role").unwrap();
        assert_eq!(role.value, "aut");
    }

    {
        let cover = doc.get_cover_id();
        assert_eq!(cover, Some("portada.png".into()));
    }

    {
        let modified = doc.mdata("dcterms:modified");
        assert_eq!(modified.unwrap().value, "2015-08-10T18:12:03Z");
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
fn doc_open_epub3() {
    let doc = EpubDoc::new("tests/docs/fatbf.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();

    {
        // Test refinements
        let mut iter = doc.metadata.iter();
        let finder = |item: &&MetadataItem| item.property == "identifier";

        let identifier = iter.find(finder).unwrap();
        assert!(identifier.refined.is_empty());

        let identifier = iter.find(finder).unwrap();
        let ident_type = identifier.refinement("identifier-type").unwrap();
        assert_eq!(ident_type.scheme, Some("onix:codelist5".to_string()));
        assert_eq!(ident_type.value, "15");
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

#[test]
fn toc_title_test() {
    let doc = EpubDoc::new("test.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();

    assert!(doc.toc_title == "Todo es mío");
}

#[test]
fn version_test() {
    let doc = EpubDoc::new("test.epub");
    assert!(doc.is_ok());
    let doc = doc.unwrap();

    assert!(doc.version == EpubVersion::Version2_0);
    assert!(doc.version < EpubVersion::Version3_0);
}
