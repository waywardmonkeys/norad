//! Error types.

use std::io::Error as IoError;
use std::path::PathBuf;

use plist::Error as PlistError;
use quick_xml::Error as XmlError;
use thiserror::Error;

use crate::write::CustomSerializationError;

/// Errors that occur while working with font objects.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// An error returned when an expected layer is missing.
    #[error("layer name '{0}' does not exist")]
    MissingLayer(String),
    /// An error returned when a layer is duplicated.
    #[error("layer name '{0}' already exists")]
    DuplicateLayer(String),
    /// An error returned when there is a duplicate glyph.
    #[error("glyph named '{glyph}' already exists in layer '{layer}'")]
    DuplicateGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
    /// An error returned when there is a missing expected glyph
    #[error("glyph '{glyph}' missing from layer '{layer}'")]
    MissingGlyph {
        /// The layer name.
        layer: String,
        /// The glyph name.
        glyph: String,
    },
}

/// An error that occurs while attempting to read a .glif file from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GlifLoadError {
    /// An [`std::io::Error`].
    #[error("failed to read file")]
    Io(#[from] IoError),
    /// A [`quick_xml::Error`].
    #[error("failed to read or parse XML structure")]
    Xml(#[from] XmlError),
    /// The .glif file was malformed.
    #[error("failed to parse glyph data")]
    Parse(#[from] ErrorKind),
    /// The glyph lib's `public.objectLibs` value was something other than a dictionary.
    #[error("the glyph lib's 'public.objectLibs' value must be a dictionary")]
    PublicObjectLibsMustBeDictionary,
    /// The entry with the given identifier within the glyph lib's `public.objectLibs` dictionary was not a dictionary.
    #[error("the glyph lib's 'public.objectLibs' entry for the object with identifier '{0}' must be a dictionary")]
    ObjectLibMustBeDictionary(String),
}

/// An error that occurs while attempting to read a UFO package from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FontLoadError {
    /// The UFO cannot be accessed.
    #[error("cannot access UFO package")]
    AccessUfoDir(#[source] IoError),
    /// The upgrade process failed to move font info data from the old lib.plist schema to the new fontinfo.plist schema.
    #[error("failed to upgrade old lib.plist to current fontinfo.plist data: {0}")]
    FontInfoV1Upconversion(FontInfoErrorKind),
    /// The upgrade process failed to convert kerning groups from old to the new UFO v3 format.
    #[error("failed to upconvert groups to the latest supported format")]
    GroupsUpconversionFailure(#[source] GroupsValidationError),
    /// The (kerning) groups in kerning.plist fail validation.
    #[error("failed to load (kerning) groups")]
    InvalidGroups(#[source] GroupsValidationError),
    /// The lib.plist file was something other than a dictionary.
    #[error("the lib.plist file must contain a dictionary (<dict>...</dict>)")]
    LibFileMustBeDictionary,
    /// Failed to load a file from the data store.
    #[error("failed to load data store")]
    LoadDataStore(#[source] StoreEntryError),
    /// Failed to load the features.fea file.
    #[error("failed to read features.fea file")]
    LoadFeatureFile(#[source] IoError),
    /// Failed to load the fontinfo.plist file.
    #[error("failed to load font info data")]
    LoadFontInfo(#[source] FontInfoLoadError),
    /// Failed to load a file from the image store.
    #[error("failed to load images store")]
    LoadImagesStore(#[source] StoreEntryError),
    /// Failed to load a specific layer.
    #[error("failed to load layer '{name}' from '{path}'")]
    LoadLayer {
        /// The layer name.
        name: String,
        /// The path to the layer.
        path: PathBuf,
        /// The underlying error.
        source: LayerLoadError,
    },
    /// The UFO does not have a default layer.
    #[error("missing the default layer ('glyphs' subdirectory)")]
    MissingDefaultLayer,
    /// The UFO does not have a default layer.
    #[error("cannot find the layercontents.plist file")]
    MissingLayerContentsFile,
    /// The UFO does not have a metainfo.plist layer.
    #[error("cannot find the metainfo.plist file")]
    MissingMetaInfoFile,
    /// Failed to parse a .plist file.
    #[error("failed to parse {name} file")]
    ParsePlist {
        /// The name of the file.
        name: &'static str,
        /// The underlying error.
        source: PlistError,
    },
    /// Norad can currently only open UFO (directory) packages.
    #[error("only UFO (directory) packages are supported")]
    UfoNotADir,
}

/// An error that occurs while attempting to read a UFO layer from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LayerLoadError {
    /// Loading a glyph from a path failed.
    #[error("failed to load glyph '{name}' from '{path}'")]
    LoadGlyph {
        /// The glyph name.
        name: String,
        /// The path to the glif file.
        path: PathBuf,
        /// The underlying error.
        source: GlifLoadError,
    },
    /// Could not find the layer's contents.plist.
    #[error("cannot find the contents.plist file")]
    MissingContentsFile,
    /// Failed to parse a .plist file.
    #[error("failed to parse {name} file")]
    ParsePlist {
        /// The name of the file.
        name: &'static str,
        /// The underlying error.
        source: PlistError,
    },
}

/// An error that occurs while attempting to read a UFO fontinfo.plist file from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FontInfoLoadError {
    /// The upgrade process failed to convert an older fontinfo.plist to newer data structures.
    #[error("failed to upgrade fontinfo.plist contents to latest UFO version data: {0}")]
    FontInfoUpconversion(FontInfoErrorKind),
    /// The UFO lib.plist's `public.objectLibs` entry for the given guideline is not a dictionary.
    #[error("the lib.plist file's 'public.objectLibs' entry for the global guideline with identifier '{0}' in the fontinfo.plist file must be a dictionary")]
    GlobalGuidelineLibMustBeDictionary(String),
    /// The fontinfo.plist file contains invalid data.
    #[error("fontinfo.plist contains invalid data: {0}")]
    InvalidData(FontInfoErrorKind),
    /// Could not parse the UFO's fontinfo.plist.
    #[error("failed to parse fontinfo.plist file")]
    ParseFontInfoFile(#[source] PlistError),
    /// The font lib's `public.objectLibs` value was something other than a dictionary.
    #[error("the lib.plist file's 'public.objectLibs' value must be a dictionary")]
    PublicObjectLibsMustBeDictionary,
}

/// An error pointing to invalid data in the font's info.
#[derive(Debug)]
#[non_exhaustive]
pub enum FontInfoErrorKind {
    /// openTypeOS2Selection contained bits 0, 5 or 6.
    DisallowedSelectionBits,
    /// Guideline identifiers were not unique within the fontinfo.plist file.
    DuplicateGuidelineIdentifiers,
    /// Found an empty WOFF element or record. If you have them, you have to fill them all in.
    EmptyWoffAttribute(&'static str),
    /// The openTypeHeadCreated had the wrong format.
    InvalidOpenTypeHeadCreatedDate,
    /// The openTypeOS2FamilyClass had out of range values.
    InvalidOs2FamilyClass,
    /// A Postscript data list had more elements than the specification allows.
    InvalidPostscriptListLength {
        /// The name of the property.
        name: &'static str,
        /// The maximum allowed number of elements.
        max_len: u8,
        /// The found number of elements.
        len: usize,
    },
    /// Unrecognized UFO v1 fontStyle field.
    UnknownFontStyle(i32),
    /// Unrecognized UFO v1 msCharSet field.
    UnknownMsCharSet(i32),
    /// Unrecognized openTypeOS2WidthClass.
    UnknownWidthClass(String),
    /// The openTypeGaspRangeRecords field was unsorted.
    UnsortedGaspEntries,
}

impl std::fmt::Display for FontInfoErrorKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FontInfoErrorKind::DisallowedSelectionBits => {
                write!(f, "openTypeOS2Selection must not contain bits 0, 5 or 6")
            }
            FontInfoErrorKind::DuplicateGuidelineIdentifiers => {
                write!(f, "guideline identifiers must be unique within the fontinfo.plist file")
            }
            FontInfoErrorKind::EmptyWoffAttribute(s) => {
                write!(f, "a '{}' element must not be empty", s)
            }
            FontInfoErrorKind::InvalidOpenTypeHeadCreatedDate => {
                write!(f, "openTypeHeadCreated must be of format 'YYYY/MM/DD HH:MM:SS'")
            }
            FontInfoErrorKind::InvalidOs2FamilyClass => {
                write!(f, "openTypeOS2FamilyClass must be two numbers in the range 0-14 and 0-15, respectively")
            }
            FontInfoErrorKind::InvalidPostscriptListLength { name, max_len, len } => {
                write!(
                    f,
                    "the Postscript field '{}' must contain at most {} items but found {}",
                    name, max_len, len
                )
            }
            FontInfoErrorKind::UnknownFontStyle(s) => {
                write!(f, "unrecognized fontStyle '{}'", s)
            }
            FontInfoErrorKind::UnknownMsCharSet(c) => {
                write!(f, "unrecognized msCharSet '{}'", c)
            }
            FontInfoErrorKind::UnknownWidthClass(w) => {
                write!(f, "unrecognized OS/2 width class '{}'", w)
            }
            FontInfoErrorKind::UnsortedGaspEntries => {
                write!(f, "openTypeGaspRangeRecords must be sorted by their rangeMaxPPEM values")
            }
        }
    }
}

/// An error representing a failure with a particular [`crate::datastore::Store`] entry.
#[derive(Debug, Error)]
#[error("store entry '{path}' is invalid")]
pub struct StoreEntryError {
    path: PathBuf,
    source: StoreError,
}

impl StoreEntryError {
    /// Returns a new [`StoreEntryError`].
    pub(crate) fn new(path: PathBuf, source: StoreError) -> Self {
        Self { path, source }
    }
}

/// An error representing a failure to insert content into a [`crate::datastore::Store`].
#[derive(Clone, Debug, Error)]
#[non_exhaustive]
pub enum StoreError {
    /// Tried to insert a path whose ancestor is in the store already, implying nesting a file under a file.
    #[error("the parent of the file is a file itself")]
    DirUnderFile,
    /// The path was empty.
    #[error("an empty path cannot be used as a key in the store")]
    EmptyPath,
    /// The path was neither plain file nor directory, but e.g. a symlink.
    #[error("only plain files and directories are allowed, no symlinks")]
    NotPlainFileOrDir,
    /// The path was absolute; only relative paths are allowed.
    #[error("the path must be relative")]
    PathIsAbsolute,
    /// The path was not a plain file, but e.g. a directory or symlink.
    #[error("only plain files are allowed, no symlinks")]
    NotPlainFile,
    /// The path contained a subdirectory; `images` is a flat directory.
    #[error("subdirectories are not allowed in the image store")]
    Subdir,
    /// The image did not have a valid PNG header.
    #[error("an image must be a valid PNG")]
    InvalidImage,
    /// Encountered an IO error while trying to load data
    #[error("encountered an IO error while trying to load content")]
    Io(#[from] std::sync::Arc<std::io::Error>),
}

/// An error representing a failure to validate UFO groups.
#[derive(Debug, Error)]
pub enum GroupsValidationError {
    /// An error returned when there is an invalid groups name.
    #[error("a kerning group name must have at least one character after the common 'public.kernN.' prefix.")]
    InvalidName,
    /// An error returned when there are overlapping kerning groups.
    #[error("the glyph '{glyph_name}' appears in more than one kerning group. Last found in '{group_name}'")]
    OverlappingKerningGroups {
        /// The glyph name.
        glyph_name: String,
        /// The group name.
        group_name: String,
    },
}

/// An error representing an invalid [`Color`] string.
///
/// [`Color`]: crate::Color
#[derive(Debug, Error)]
#[error("invalid color string '{string}'")]
pub struct InvalidColorString {
    /// The source string that caused the error.
    string: String,
}

impl InvalidColorString {
    pub(crate) fn new(source: String) -> Self {
        InvalidColorString { string: source }
    }
}

/// An error returned when there is an inappropriate negative sign on a value.
#[derive(Debug, Error)]
#[error("expected a positive value")]
pub struct ExpectedPositiveValue;

/// An error returned when there is a problem with kurbo contour conversion.
#[cfg(feature = "kurbo")]
#[derive(Debug, Error)]
#[error("failed to convert contour: '{0}'")]
pub struct ConvertContourError(ErrorKind);

#[cfg(feature = "kurbo")]
impl ConvertContourError {
    pub(crate) fn new(kind: ErrorKind) -> Self {
        ConvertContourError(kind)
    }
}

/// An error that occurs while attempting to write a UFO package to disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum FontWriteError {
    /// Cannot clean up previous UFO package before writing out new one.
    #[error("failed to remove target directory before overwriting")]
    Cleanup(#[source] IoError),
    /// Failed to create the data directory.
    #[error("failed to create target data directory '{path}'")]
    CreateDataDir {
        /// The path to the entry.
        path: PathBuf,
        /// The underlying error.
        source: IoError,
    },
    /// Failed to create the images directory.
    #[error("failed to create target image directory '{path}'")]
    CreateImageDir {
        /// The path to the entry.
        path: PathBuf,
        /// The underlying error.
        source: IoError,
    },
    /// Failed to create the UFO package directory.
    #[error("failed to create target font directory")]
    CreateUfoDir(#[source] IoError),
    /// Norad does not currently support downgrading to older UFO formats.
    #[error("downgrading below UFO v3 is not currently supported")]
    Downgrade,
    /// The font info contains invalid data.
    #[error("font info contains invalid data: {0}")]
    InvalidFontInfo(FontInfoErrorKind),
    /// The groups contains invalid data.
    #[error("failed to write (kerning) groups")]
    InvalidGroups(#[source] GroupsValidationError),
    /// The data or images store contains invalid data.
    #[error("store entry '{path}' is invalid")]
    InvalidStoreEntry {
        /// The path to the entry.
        path: PathBuf,
        /// The underlying error.
        source: StoreError,
    },
    /// There exists a `public.objectLibs` lib key when it should be set only by norad.
    #[error("the `public.objectLibs` lib key is managed by norad and must not be set manually")]
    PreexistingPublicObjectLibsKey,
    /// Failed to write out a customly-serialized file.
    #[error("failed to write {name} file")]
    WriteCustomFile {
        /// The name of the file.
        name: &'static str,
        /// The underlying error.
        source: CustomSerializationError,
    },
    /// Failed to write data entry.
    #[error("failed to write data file")]
    WriteData {
        /// The path to the entry.
        path: PathBuf,
        /// The underlying error.
        source: IoError,
    },
    /// Failed to write out the feature.fea file.
    #[error("failed to write feature.fea file")]
    WriteFeatureFile(#[source] IoError),
    /// Failed to write out an image file.
    #[error("failed to write image file")]
    WriteImage {
        /// The path to the entry.
        path: PathBuf,
        /// The underlying error.
        source: IoError,
    },
    /// Failed to write out a layer.
    #[error("failed to write layer '{name}' to '{path}'")]
    WriteLayer {
        /// The name of the layer.
        name: String,
        /// The path to the layer.
        path: PathBuf,
        /// The underlying error.
        source: LayerWriteError,
    },
}

/// An error that occurs while attempting to read a UFO layer from disk.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LayerWriteError {
    /// Failed to create the layer's directory.
    #[error("cannot create layer directory")]
    CreateDir(#[source] IoError),
    /// Failed to write out the contents.plist file
    #[error("failed to write contents.plist file")]
    WriteContents(#[source] CustomSerializationError),
    /// Failed to write out a glyph.
    #[error("failed to write glyph '{name}' to '{path}'")]
    WriteGlyph {
        /// The name of the glyph.
        name: String,
        /// The path to the glyph.
        path: PathBuf,
        /// The underlying error.
        source: GlifWriteError,
    },
    /// Failed to write out the layerinfo.plist file
    #[error("failed to write layerinfo.plist file")]
    WriteLayerInfo(#[source] CustomSerializationError),
}

/// An error when attempting to write a .glif file.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum GlifWriteError {
    /// Failed to serialize a glyph to an internal buffer.
    #[error("failed to serialize glyph to an internal buffer")]
    Buffer(#[source] IoError),
    /// Norad does not currently support downgrading format versions.
    #[error("downgrading below glyph format version 2 is unsupported")]
    Downgrade,
    /// When writing out the 'lib' section, we use the plist crate to generate
    /// the plist xml, and then strip the preface and closing </plist> tag.
    ///
    /// If for some reason the implementation of that crate changes, we could
    /// be affected, although this is very unlikely.
    #[error("internal error while writing lib data, please open an issue")]
    InternalLibWriteError,
    /// Failed to write a .glif file to disk.
    #[error("failed to write .glif file")]
    Io(#[source] IoError),
    /// Plist serialization error. Wraps a [PlistError].
    #[error("error serializing glyph lib data internally")]
    Plist(#[source] PlistError),
    /// There exists a `public.objectLibs` glyph lib key when it should be set only by norad.
    #[error("the `public.objectLibs` lib key is managed by norad and must not be set manually")]
    PreexistingPublicObjectLibsKey,
    /// XML serialization error. Wraps a [XmlError].
    #[error("error serializing glyph to XML")]
    Xml(#[source] XmlError),
}

/// The reason for a glif parse failure.
#[derive(Debug, Clone, Copy, Error)]
pub enum ErrorKind {
    /// The glif version is not supported by this library.
    #[error("unsupported glif version")]
    UnsupportedGlifVersion,
    /// An unknown point type.
    #[error("unknown point type")]
    UnknownPointType,
    /// The first XML element of a glif file is invalid.
    #[error("wrong first XML element in glif file")]
    WrongFirstElement,
    /// Missing a close tag.
    #[error("missing close tag")]
    MissingCloseTag,
    /// Has an unexpected tag.
    #[error("unexpected tag")]
    UnexpectedTag,
    /// Has an invalid hexadecimal value.
    #[error("bad hex value")]
    BadHexValue,
    /// Has an invalid numeric value.
    #[error("bad number")]
    BadNumber,
    /// Has an invalid color value.
    #[error("bad color")]
    BadColor,
    /// Has an invalid anchor definition.
    #[error("bad anchor")]
    BadAnchor,
    /// Has an invalid point definition.
    #[error("bad point")]
    BadPoint,
    /// Has an invalid guideline definition.
    #[error("bad guideline")]
    BadGuideline,
    /// Has an invalid component definition.
    #[error("bad component")]
    BadComponent,
    /// Has an invalid image definition.
    #[error("bad image")]
    BadImage,
    /// Has an invalid identifier.
    #[error("bad identifier")]
    BadIdentifier,
    /// Has an invalid lib.
    #[error("bad lib")]
    BadLib,
    /// Has an unexected duplicate value.
    #[error("unexpected duplicate")]
    UnexpectedDuplicate,
    /// Has an unexpected move definition.
    #[error("unexpected move point, can only occur at start of contour")]
    UnexpectedMove,
    /// Has an unexpected smooth definition.
    #[error("unexpected smooth attribute on an off-curve point")]
    UnexpectedSmooth,
    /// Has an unexpected element definition.
    #[error("unexpected element")]
    UnexpectedElement,
    /// Has an unexpected attribute definition.
    #[error("unexpected attribute")]
    UnexpectedAttribute,
    /// Has an unexpected end of file definition.
    #[error("unexpected EOF")]
    UnexpectedEof,
    /// Has an unexpected point following an off curve point definition.
    #[error("an off-curve point must be followed by a curve or qcurve")]
    UnexpectedPointAfterOffCurve,
    /// Has too many off curve points in sequence.
    #[error("at most two off-curve points can precede a curve")]
    TooManyOffCurves,
    /// The contour pen path was not started
    #[error("must call begin_path() before calling add_point() or end_path()")]
    PenPathNotStarted,
    /// Has trailing off curve points defined.
    #[error("open contours must not have trailing off-curves")]
    TrailingOffCurves,
    /// Has duplicate identifiers.
    #[error("duplicate identifier")]
    DuplicateIdentifier,
    /// Has unexepected drawing data.
    #[error("unexpected drawing without an outline")]
    UnexpectedDrawing,
    /// Has incomplete drawing data.
    #[error("unfinished drawing, you must call end_path")]
    UnfinishedDrawing,
    /// Has an unexpected point field.
    #[error("unexpected point field")]
    UnexpectedPointField,
    /// Has an unexpected component field.
    #[error("unexpected component field")]
    UnexpectedComponentField,
    /// Has an unexpected anchor field.
    #[error("unexpected anchor field")]
    UnexpectedAnchorField,
    /// Has an unexpected guideline field.
    #[error("unexpected guideline field")]
    UnexpectedGuidelineField,
    /// Has an unexpected image field.
    #[error("unexpected image field")]
    UnexpectedImageField,
}

#[doc(hidden)]
impl From<IoError> for StoreError {
    fn from(src: IoError) -> StoreError {
        StoreError::Io(std::sync::Arc::new(src))
    }
}
