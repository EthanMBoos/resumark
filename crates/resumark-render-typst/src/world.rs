use std::path::PathBuf;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

const MAIN_PATH: &str = "/main.typ";
const DOCUMENT_PATH: &str = "/resume.json";
const THEME_SOURCE: &str = include_str!("../../../themes/minimal.typ");

pub(crate) struct ResumarkWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    main: Source,
    document_id: FileId,
    document_json: Bytes,
    fonts: Vec<Font>,
}

impl ResumarkWorld {
    pub(crate) fn new(document_json: Vec<u8>, fonts: Vec<Font>) -> Option<Self> {
        let main_id = file_id(MAIN_PATH)?;
        let document_id = file_id(DOCUMENT_PATH)?;
        let book = FontBook::from_fonts(&fonts);

        Some(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            main: Source::new(main_id, THEME_SOURCE.to_owned()),
            document_id,
            document_json: Bytes::new(document_json),
            fonts,
        })
    }
}

impl World for ResumarkWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main.id()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main.id() {
            return Ok(self.main.clone());
        }

        Err(not_found(id))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if id == self.main.id() {
            return Ok(Bytes::from_string(self.main.text().to_owned()));
        }

        if id == self.document_id {
            return Ok(self.document_json.clone());
        }

        Err(not_found(id))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<Duration>) -> Option<Datetime> {
        None
    }
}

fn file_id(path: &str) -> Option<FileId> {
    let virtual_path = VirtualPath::new(path).ok()?;
    Some(FileId::new(RootedPath::new(
        VirtualRoot::Project,
        virtual_path,
    )))
}

fn not_found(id: FileId) -> FileError {
    FileError::NotFound(PathBuf::from(id.vpath().get_with_slash()))
}
