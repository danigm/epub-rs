use epub::archive::EpubArchive;
use std::fs;
use std::io::Write;

#[test]
fn archive_open() {
    let archive = EpubArchive::new("test.epub");
    assert!(archive.is_ok());
    let archive = archive.unwrap();
    assert_eq!("test.epub", archive.path.display().to_string());
    assert_eq!(32, archive.files.len());
}

#[test]
fn archive_entry() {
    let archive = EpubArchive::new("test.epub");
    assert!(archive.is_ok());
    let mut archive = archive.unwrap();
    let content = archive.get_entry("META-INF/container.xml");
    assert!(content.is_ok());
}

#[test]
fn archive_entry_percent_encoding() {
    let archive = EpubArchive::new("test.epub");
    assert!(archive.is_ok());
    let mut archive = archive.unwrap();
    let content = archive.get_entry("a%20%25%20encoded%20item.xml");
    assert!(content.is_ok());
    let content = archive.get_entry("a%20normal%20item.xml");
    assert!(content.is_ok());
}

#[test]
fn archive_root_file() {
    let archive = EpubArchive::new("test.epub");
    assert!(archive.is_ok());
    let mut archive = archive.unwrap();
    let content = archive.get_entry("META-INF/container.xml");
    let root = archive.get_container_file();
    assert!(content.is_ok() && root.is_ok());
    assert_eq!(content.unwrap(), root.unwrap());
}

#[test]
#[ignore]
fn archive_bin_entry() {
    let archive = EpubArchive::new("test.epub");
    assert!(archive.is_ok());
    let mut archive = archive.unwrap();
    let content = archive.get_entry("OEBPS/Images/portada.png");
    assert!(content.is_ok());

    let content = content.unwrap();
    let f = fs::File::create("cover.png");
    assert!(f.is_ok());
    let mut f = f.unwrap();
    let resp = f.write_all(&content);
    assert!(resp.is_ok());
}
