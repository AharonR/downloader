//! Corpus export: generate BibTeX or RIS bibliography files from JSON-LD sidecar metadata.
//!
//! This module scans a downloaded corpus directory for `.json` sidecar files produced by
//! [`crate::sidecar`], deserializes them as Schema.org `ScholarlyArticle` records, and
//! renders them as a bibliography in either BibTeX (`.bib`) or RIS (`.ris`) format.
//!
//! # Typical usage
//!
//! ```no_run
//! use std::path::Path;
//! use downloader_core::export::{ExportFormat, scan_corpus, generate_bibtex, generate_ris};
//!
//! let entries = scan_corpus(Path::new("/my/corpus")).unwrap();
//! let bib = generate_bibtex(&entries);
//! std::fs::write("bibliography.bib", bib).unwrap();
//! ```

mod bibtex;
mod error;
mod ris;
mod sidecar_reader;

pub use bibtex::generate_bibtex;
pub use error::ExportError;
pub use ris::generate_ris;
pub use sidecar_reader::{SidecarAuthor, SidecarEntry, SidecarIdentifier, scan_corpus};

/// Output format for corpus export.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// BibTeX format (`.bib`), importable by Zotero, `JabRef`, and most reference managers.
    BibTex,
    /// RIS format (`.ris`), importable by Zotero, Mendeley, `EndNote`, and most reference managers.
    Ris,
}
