use std::path::PathBuf;

use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime, Duration};
use typst::syntax::{FileId, RootedPath, Source, VirtualPath, VirtualRoot};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

const MAIN_PATH: &str = "/main.typ";
const THEME_PATH: &str = "/theme.typ";
const API_PATH: &str = "/resumark/v1.typ";
const DOCUMENT_PATH: &str = "/resume.json";

const MAIN_SOURCE: &str = include_str!("../../../themes/main.typ");
const API_SOURCE: &str = include_str!("../../../themes/resumark-v1.typ");

pub(crate) struct ResumarkWorld {
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    main: Source,
    theme: Source,
    api: Source,
    document_id: FileId,
    document_json: Bytes,
    fonts: Vec<Font>,
}

impl ResumarkWorld {
    pub(crate) fn new(
        document_json: Vec<u8>,
        theme_source: String,
        fonts: Vec<Font>,
    ) -> Option<Self> {
        let main_id = file_id(MAIN_PATH)?;
        let theme_id = file_id(THEME_PATH)?;
        let api_id = file_id(API_PATH)?;
        let document_id = file_id(DOCUMENT_PATH)?;
        let book = FontBook::from_fonts(&fonts);

        Some(Self {
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(book),
            main: Source::new(main_id, MAIN_SOURCE.to_owned()),
            theme: Source::new(theme_id, theme_source),
            api: Source::new(api_id, API_SOURCE.to_owned()),
            document_id,
            document_json: Bytes::new(document_json),
            fonts,
        })
    }

    pub(crate) fn theme_id(&self) -> FileId {
        self.theme.id()
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
        if id == self.theme.id() {
            return Ok(self.theme.clone());
        }
        if id == self.api.id() {
            return Ok(self.api.clone());
        }

        Err(not_found(id))
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        if let Ok(source) = self.source(id) {
            return Ok(Bytes::from_string(source.text().to_owned()));
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

#[cfg(test)]
mod tests {
    use typst::syntax::package::PackageSpec;

    use super::*;

    #[test]
    fn world_exposes_only_the_render_bundle() {
        let world = ResumarkWorld::new(
            b"{}".to_vec(),
            "#let render(..args) = none".into(),
            Vec::new(),
        )
        .expect("the bundled virtual paths are valid");

        assert!(world.source(world.main.id()).is_ok());
        assert!(world.source(world.theme.id()).is_ok());
        assert!(world.source(world.api.id()).is_ok());
        assert!(world.file(world.document_id).is_ok());

        let unknown = file_id("/private.txt").expect("the test path is valid");
        assert!(matches!(world.source(unknown), Err(FileError::NotFound(_))));
        assert!(matches!(world.file(unknown), Err(FileError::NotFound(_))));
    }

    #[test]
    fn world_rejects_package_files() {
        let world = ResumarkWorld::new(
            b"{}".to_vec(),
            "#let render(..args) = none".into(),
            Vec::new(),
        )
        .expect("the bundled virtual paths are valid");
        let package = "@preview/example:1.0.0"
            .parse::<PackageSpec>()
            .expect("the test package spec is valid");
        let path = VirtualPath::new("/main.typ").expect("the test package path is valid");
        let package_id = FileId::new(RootedPath::new(VirtualRoot::Package(package), path));

        assert!(matches!(
            world.source(package_id),
            Err(FileError::NotFound(_))
        ));
        assert!(matches!(
            world.file(package_id),
            Err(FileError::NotFound(_))
        ));
    }
}
