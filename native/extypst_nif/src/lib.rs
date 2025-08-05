use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::Hash;
use std::path::{Path, PathBuf};
use std::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};

use comemo::LazyHash;
use elsa::sync::FrozenVec;
use memmap2::Mmap;
use once_cell::sync::OnceCell;
use same_file::Handle;
use siphasher::sip128::{Hasher128, SipHasher13};
use typst::diag::{FileError, FileResult, StrResult};
use typst::foundations::Bytes;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook, FontInfo};
use typst::{Library, World};
use walkdir::WalkDir;

/// A world that provides access to the operating system.
pub struct SystemWorld {
    root: PathBuf,
    library: LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<FontSlot>,
    hashes: RwLock<HashMap<PathBuf, FileResult<PathHash>>>,
    paths: RwLock<HashMap<PathHash, PathSlot>>,
    sources: FrozenVec<Box<Source>>,
    main: FileId,
}

/// Holds details about the location of a font and lazily the font itself.
#[derive(Debug)]
struct FontSlot {
    path: PathBuf,
    index: u32,
    font: OnceCell<Option<Font>>,
}

/// Holds canonical data for all paths pointing to the same entity.
#[derive(Default)]
struct PathSlot {
    source: OnceCell<FileResult<FileId>>,
    buffer: OnceCell<FileResult<Bytes>>,
}

impl World for SystemWorld {
    fn library(&self) -> &LazyHash<Library> {
        &self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if let Some(source) = self.sources.get(id.as_u16() as usize) {
            Ok(source.as_ref().clone())
        } else {
            Err(FileError::NotFound(PathBuf::from("source not found")))
        }
    }

    fn file(&self, _id: FileId) -> FileResult<Bytes> {
        // Simplified implementation - just return empty bytes
        // In a real implementation, you'd map FileId to actual file content
        Ok(Bytes::new(vec![]))
    }

    fn font(&self, index: usize) -> Option<Font> {
        let slot = self.fonts.get(index)?;

        slot.font
            .get_or_init(|| {
                let data = read(&slot.path).ok()?;
                Font::new(Bytes::new(data), slot.index)
            })
            .clone()
    }

    fn today(&self, _offset: Option<i64>) -> Option<typst::foundations::Datetime> {
        None // Simple implementation, could be enhanced
    }
}

impl SystemWorld {
    pub fn new(root: PathBuf, font_paths: &[PathBuf], font_files: &[PathBuf]) -> Self {
        let mut searcher = FontSearcher::new();
        searcher.search_system();

        for path in font_paths {
            searcher.search_dir(path);
        }
        for path in font_files {
            searcher.search_file(path);
        }

        Self {
            root,
            library: LazyHash::new(Library::default()),
            book: LazyHash::new(searcher.book),
            fonts: searcher.fonts,
            hashes: RwLock::default(),
            paths: RwLock::default(),
            sources: FrozenVec::new(),
            main: FileId::new(None, VirtualPath::new("MARKUP.typ")),
        }
    }

    // Simplified slot management - removed for now to avoid lifetime issues

    fn insert(&self, path: &Path, text: String) -> FileId {
        let id = FileId::new(None, VirtualPath::new(path));
        let source = Source::new(id, text);
        self.sources.push(Box::new(source));
        id
    }

    fn reset(&mut self) {
        // Clear sources
        // Note: FrozenVec doesn't have a clear method, so we'll need a different approach
        self.hashes.write().unwrap().clear();
        self.paths.write().unwrap().clear();
    }

    pub fn compile(&mut self, markup: String) -> StrResult<Vec<u8>> {
        self.reset();
        self.main = self.insert(Path::new("MARKUP.typ"), markup);

        match typst::compile(self).output {
            // Export the PDF.
            Ok(document) => {
                let buffer = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())?;
                Ok(buffer)
            }

            // Format diagnostics.
            Err(errors) => {
                let mut error_msg = "compile error:\n".to_string();
                for error in errors.iter() {
                    error_msg.push_str(&format!("{}", error.message));
                    // For simplicity, we're not including detailed range information
                    // as the API for extracting ranges has changed
                }
                Err(error_msg.into())
            }
        }
    }
}

/// A hash that is the same for all paths pointing to the same entity.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
struct PathHash(u128);

impl PathHash {
    fn new(path: &Path) -> FileResult<Self> {
        let f = |e| FileError::from_io(e, path);
        let handle = Handle::from_path(path).map_err(f)?;
        let mut state = SipHasher13::new();
        handle.hash(&mut state);
        Ok(Self(state.finish128().as_u128()))
    }
}

/// Read a file.
fn read(path: &Path) -> FileResult<Vec<u8>> {
    let f = |e| FileError::from_io(e, path);
    if fs::metadata(path).map_err(f)?.is_dir() {
        Err(FileError::IsDirectory)
    } else {
        fs::read(path).map_err(f)
    }
}

/// Searches for fonts.
struct FontSearcher {
    book: FontBook,
    fonts: Vec<FontSlot>,
}

impl FontSearcher {
    /// Create a new, empty system searcher.
    fn new() -> Self {
        Self {
            book: FontBook::new(),
            fonts: vec![],
        }
    }

    /// Search for fonts in the linux system font directories.
    #[cfg(all(unix, not(target_os = "macos")))]
    fn search_system(&mut self) {
        self.search_dir("/usr/share/fonts");
        self.search_dir("/usr/local/share/fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for fonts in the macOS system font directories.
    #[cfg(target_os = "macos")]
    fn search_system(&mut self) {
        self.search_dir("/Library/Fonts");
        self.search_dir("/System/Library/Fonts");

        // Downloadable fonts, location varies on major macOS releases
        if let Ok(dir) = fs::read_dir("/System/Library/AssetsV2") {
            for entry in dir {
                let Ok(entry) = entry else { continue };
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with("com_apple_MobileAsset_Font")
                {
                    self.search_dir(entry.path());
                }
            }
        }

        self.search_dir("/Network/Library/Fonts");

        if let Some(dir) = dirs::font_dir() {
            self.search_dir(dir);
        }
    }

    /// Search for fonts in the Windows system font directories.
    #[cfg(windows)]
    fn search_system(&mut self) {
        let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());

        self.search_dir(Path::new(&windir).join("Fonts"));

        if let Some(roaming) = dirs::config_dir() {
            self.search_dir(roaming.join("Microsoft\\Windows\\Fonts"));
        }

        if let Some(local) = dirs::cache_dir() {
            self.search_dir(local.join("Microsoft\\Windows\\Fonts"));
        }
    }

    /// Search for all fonts in a directory recursively.
    fn search_dir(&mut self, path: impl AsRef<Path>) {
        for entry in WalkDir::new(path)
            .follow_links(true)
            .sort_by(|a, b| a.file_name().cmp(b.file_name()))
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if matches!(
                path.extension().and_then(|s| s.to_str()),
                Some("ttf" | "otf" | "TTF" | "OTF" | "ttc" | "otc" | "TTC" | "OTC"),
            ) {
                self.search_file(path);
            }
        }
    }

    /// Index the fonts in the file at the given path.
    fn search_file(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        if let Ok(file) = File::open(path) {
            if let Ok(mmap) = unsafe { Mmap::map(&file) } {
                for (i, info) in FontInfo::iter(&mmap).enumerate() {
                    self.book.push(info);
                    self.fonts.push(FontSlot {
                        path: path.into(),
                        index: i as u32,
                        font: OnceCell::new(),
                    });
                }
            }
        }
    }
}


#[rustler::nif]
fn compile<'a>(markup: String, extra_fonts: Vec<String>) -> Result<String, String> {
    let extra_fonts_paths: Vec<PathBuf> = extra_fonts.iter().map(|f| Path::new(f).into()).collect();
   
    let mut world = SystemWorld::new(".".into(), extra_fonts_paths.as_slice(), &[]);
    let result = match world.compile(markup) {
        Ok(pdf_bytes) => {
            // the resulting string is not an utf-8 encoded string, but this is exactly what we
            // want as we are passing a binary back to elixir
            unsafe {
                return Ok(String::from_utf8_unchecked(pdf_bytes));
            }
        },
        Err(e) => Err(e.into())
    };

    result
}

rustler::init!("Elixir.ExTypst.NIF", [compile]);
