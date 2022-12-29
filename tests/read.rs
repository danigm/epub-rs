use epub::doc::EpubDoc;

#[test]
fn read_doc() {
    let input_file = "tests/docs/Metamorphosis-jackson.epub";
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let mut doc = doc.unwrap();

    if let Some(title) = doc.mdata("title") {
        println!("Book title: {}", title);
    } else {
        println!("Book title not found");
    }
    println!("Num Pages: {}\n", doc.get_num_pages());

    {
        println!("resources:\n");
        for (k, v) in doc.resources.iter() {
            println!("{}: {}\n * {}\n", k, v.1, v.0.display());
        }
        println!();
    }

    while doc.go_next() {
        println!("ID: {}", doc.get_current_id().unwrap());
        let current = doc.get_current_str();
        match current {
            Some((v, m)) => println!("Value {:?}, Mime {:?}\n", v, m),
            None => println!("Not Found\n"),
        }
    }
}

#[test]
fn bad_epub() {
    //book2.epub has a opf encoded in UTF-16
    //It also has malformed toc, manifest and guide entries, as well as multiple metadata entries
    let input_file = "tests/docs/book2.epub";
    let doc = EpubDoc::new(input_file);
    assert!(doc.is_ok());
    let doc = doc.unwrap();
    if let Some(titles) = doc.metadata.get("title") {
        assert_eq!(
            titles,
            &vec!["Metamorphosis ".to_string(), "Metamorphosis2 ".to_string()]
        );
        println!("Book title: {:#?}", titles);
    } else {
        println!("Book title not found");
    }
}
