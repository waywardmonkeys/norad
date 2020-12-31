//! Data related to individual glyphs.

pub mod builder;
mod parse;
mod serialize;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(feature = "druid")]
use druid::{Data, Lens};

use crate::error::{Error, ErrorKind, GlifError, GlifErrorInternal};
use crate::names::NameList;
use crate::shared_types::{Color, Guideline, Identifier, Line, Plist, PUBLIC_OBJECT_LIBS_KEY};

/// The name of a glyph.
pub type GlyphName = Arc<str>;

/// A glyph, loaded from a [.glif file][glif].
///
/// [glif]: http://unifiedfontobject.org/versions/ufo3/glyphs/glif/
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Lens))]
pub struct Glyph {
    pub name: GlyphName,
    pub format: GlifVersion,
    pub advance: Option<Advance>,
    pub codepoints: Option<Vec<char>>,
    pub note: Option<String>,
    pub guidelines: Option<Vec<Guideline>>,
    pub anchors: Option<Vec<Anchor>>,
    pub outline: Option<Outline>,
    pub image: Option<Image>,
    pub lib: Option<Plist>,
}

impl Glyph {
    /// Load the glyph at this path.
    ///
    /// When loading glyphs in bulk, `load_with_names` should be preferred,
    /// since it will allow glyph names (in glyphs and components) to be shared
    /// between instances.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Error> {
        let path = path.as_ref();
        let names = NameList::default();
        Glyph::load_with_names(path, &names)
    }

    pub fn load_with_names(path: &Path, names: &NameList) -> Result<Self, Error> {
        let data = std::fs::read(path)?;
        parse::GlifParser::from_xml(&data, Some(names)).map_err(|e| match e {
            GlifErrorInternal::Xml(e) => e.into(),
            GlifErrorInternal::Spec { kind, position } => {
                GlifError { kind, position, path: Some(path.to_owned()) }.into()
            }
        })
    }

    #[doc(hidden)]
    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), Error> {
        if self.format != GlifVersion::V2 {
            return Err(Error::DowngradeUnsupported);
        }
        if let Some(lib) = &self.lib {
            if lib.contains_key(PUBLIC_OBJECT_LIBS_KEY) {
                return Err(Error::PreexistingPublicObjectLibsKey);
            }
        }
        let data = self.encode_xml()?;
        std::fs::write(path, &data)?;
        Ok(())
    }

    /// Create a new glyph with the given name.
    pub fn new_named<S: Into<GlyphName>>(name: S) -> Self {
        Glyph::new(name.into(), GlifVersion::V2)
    }

    /// If this glyph has an advance, return the width value.
    ///
    /// This is purely a convenience method.
    pub fn advance_width(&self) -> Option<f32> {
        self.advance.as_ref().map(|adv| adv.width)
    }

    pub(crate) fn new(name: GlyphName, format: GlifVersion) -> Self {
        Glyph {
            name,
            format,
            advance: None,
            codepoints: None,
            note: None,
            guidelines: None,
            anchors: None,
            outline: None,
            image: None,
            lib: None,
        }
    }

    /// Move libs from the lib's `public.objectLibs` into the actual objects.
    /// The key will be removed from the glyph lib.
    fn fill_in_libs(&mut self) -> Result<(), ErrorKind> {
        let object_libs = match self.lib.as_mut().and_then(|lib| lib.remove(PUBLIC_OBJECT_LIBS_KEY))
        {
            Some(lib) => lib.into_dictionary().ok_or(ErrorKind::BadLib)?,
            None => return Ok(()),
        };

        'next_key: for (key, value) in object_libs.into_iter() {
            let value = value.into_dictionary().ok_or(ErrorKind::BadLib)?;

            if let Some(anchors) = &mut self.anchors {
                for anchor in anchors {
                    if anchor.identifier().map(Identifier::as_str) == Some(&key) {
                        anchor.replace_lib(value);
                        continue 'next_key;
                    }
                }
            }
            if let Some(guidelines) = &mut self.guidelines {
                for guideline in guidelines {
                    if guideline.identifier().map(Identifier::as_str) == Some(&key) {
                        guideline.replace_lib(value);
                        continue 'next_key;
                    }
                }
            }
            if let Some(outline) = &mut self.outline {
                for contour in &mut outline.contours {
                    if contour.identifier().map(Identifier::as_str) == Some(&key) {
                        contour.replace_lib(value);
                        continue 'next_key;
                    }
                    for point in &mut contour.points {
                        if point.identifier().map(Identifier::as_str) == Some(&key) {
                            point.replace_lib(value);
                            continue 'next_key;
                        }
                    }
                }
                for component in &mut outline.components {
                    if component.identifier().map(Identifier::as_str) == Some(&key) {
                        component.replace_lib(value);
                        continue 'next_key;
                    }
                }
            }
        }

        Ok(())
    }

    /// Dump guideline libs into a Plist.
    fn libs_to_object_libs(&self) -> Plist {
        let mut object_libs = Plist::default();

        let mut add_lib = |id: Option<&Identifier>, lib: &Plist| {
            let id = id.map(|id| id.as_str().to_string());
            object_libs.insert(id.unwrap(), plist::Value::Dictionary(lib.clone()));
        };

        if let Some(anchors) = &self.anchors {
            for anchor in anchors {
                if let Some(lib) = anchor.lib() {
                    add_lib(anchor.identifier(), lib);
                }
            }
        }

        if let Some(guidelines) = &self.guidelines {
            for guideline in guidelines {
                if let Some(lib) = guideline.lib() {
                    add_lib(guideline.identifier(), lib);
                }
            }
        }

        if let Some(outline) = &self.outline {
            for contour in &outline.contours {
                if let Some(lib) = contour.lib() {
                    add_lib(contour.identifier(), lib);
                }
                for point in &contour.points {
                    if let Some(lib) = point.lib() {
                        add_lib(point.identifier(), lib);
                    }
                }
            }
            for component in &outline.components {
                if let Some(lib) = component.lib() {
                    add_lib(component.identifier(), lib);
                }
            }
        }

        object_libs
    }
}

#[cfg(feature = "druid")]
impl Data for Glyph {
    fn same(&self, other: &Glyph) -> bool {
        self.name.same(&other.name)
            && self.format.same(&other.format)
            && self.advance.same(&other.advance)
            && self.codepoints == other.codepoints
            && self.note == other.note
            && self.guidelines == other.guidelines
            && self.anchors == other.anchors
            && self.outline == other.outline
            && self.image == other.image
            && self.lib == other.lib
    }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub enum GlifVersion {
    V1 = 1,
    V2 = 2,
}

/// Horizontal and vertical metrics.
#[derive(Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct Advance {
    pub height: f32,
    pub width: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Anchor {
    pub x: f32,
    pub y: f32,
    /// An arbitrary name for the anchor.
    pub name: Option<String>,
    pub color: Option<Color>,
    /// Unique identifier for the anchor within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The anchor's lib for arbitary data.
    lib: Option<Plist>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Outline {
    pub components: Vec<Component>,
    pub contours: Vec<Contour>,
}

/// Another glyph inserted as part of the outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Component {
    /// The name of the base glyph.
    pub base: GlyphName,
    pub transform: AffineTransform,
    /// Unique identifier for the component within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The component's lib for arbitary data.
    lib: Option<Plist>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Contour {
    pub points: Vec<ContourPoint>,
    /// Unique identifier for the contour within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The contour's lib for arbitary data.
    lib: Option<Plist>,
}

impl Contour {
    fn is_closed(&self) -> bool {
        self.points.first().map_or(true, |v| v.typ != PointType::Move)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContourPoint {
    pub x: f32,
    pub y: f32,
    pub typ: PointType,
    pub smooth: bool,
    pub name: Option<String>,
    /// Unique identifier for the point within the glyph. This attribute is only required
    /// when a lib is present and should otherwise only be added as needed.
    identifier: Option<Identifier>,
    /// The point's lib for arbitary data.
    lib: Option<Plist>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PointType {
    /// A point of this type must be the first in a contour. The reverse is not true:
    /// a contour does not necessarily start with a move point. When a contour
    /// does start with a move point, it signifies the beginning of an open contour.
    /// A closed contour does not start with a move and is defined as a cyclic
    /// list of points, with no predominant start point. There is always a next
    /// point and a previous point. For this purpose the list of points can be
    /// seen as endless in both directions. The actual list of points can be
    /// rotated arbitrarily (by removing the first N points and appending
    /// them at the end) while still describing the same outline.
    Move,
    /// Draw a straight line from the previous point to this point.
    /// The previous point must be a move, a line, a curve or a qcurve.
    /// It must not be an offcurve.
    Line,
    /// This point is part of a curve segment that goes up to the next point
    /// that is either a curve or a qcurve.
    OffCurve,
    /// Draw a cubic bezier curve from the last non-offcurve point to this point.
    /// The number of offcurve points can be zero, one or two.
    /// If the number of offcurve points is zero, a straight line is drawn.
    /// If it is one, a quadratic curve is drawn.
    /// If it is two, a regular cubic bezier is drawn.
    Curve,
    /// Similar to curve, but uses quadratic curves, using the TrueType
    /// “implied on-curve points” principle.
    QCurve,
}

/// Taken together in order, these fields represent an affine transformation matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "druid", derive(Data))]
pub struct AffineTransform {
    pub x_scale: f32,
    pub xy_scale: f32,
    pub yx_scale: f32,
    pub y_scale: f32,
    pub x_offset: f32,
    pub y_offset: f32,
}

impl Anchor {
    pub fn new(
        x: f32,
        y: f32,
        name: Option<String>,
        color: Option<Color>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        Self { x, y, name, color, identifier, lib }
    }

    /// Returns an immutable reference to the anchor's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the anchor's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the anchor, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns an immutable reference to the anchor's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

impl Contour {
    pub fn new(
        points: Vec<ContourPoint>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        Self { points, identifier, lib }
    }

    /// Returns an immutable reference to the contour's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the contour's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the contour, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns an immutable reference to the contour's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

impl ContourPoint {
    pub fn new(
        x: f32,
        y: f32,
        typ: PointType,
        smooth: bool,
        name: Option<String>,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        Self { x, y, typ, smooth, name, identifier, lib }
    }

    /// Returns an immutable reference to the contour's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the contour's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the contour, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns an immutable reference to the contour's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

impl Component {
    pub fn new(
        base: GlyphName,
        transform: AffineTransform,
        identifier: Option<Identifier>,
        lib: Option<Plist>,
    ) -> Self {
        Self { base, transform, identifier, lib }
    }

    /// Returns an immutable reference to the component's lib.
    pub fn lib(&self) -> Option<&Plist> {
        self.lib.as_ref()
    }

    /// Returns a mutable reference to the component's lib.
    pub fn lib_mut(&mut self) -> Option<&mut Plist> {
        self.lib.as_mut()
    }

    /// Replaces the actual lib by the lib given in parameter, returning the old
    /// lib if present. Sets a new UUID v4 identifier if none is set already.
    pub fn replace_lib(&mut self, lib: Plist) -> Option<Plist> {
        if self.identifier.is_none() {
            self.identifier.replace(Identifier::from_uuidv4());
        }
        self.lib.replace(lib)
    }

    /// Takes the lib out of the component, leaving a None in its place.
    pub fn take_lib(&mut self) -> Option<Plist> {
        self.lib.take()
    }

    /// Returns an immutable reference to the component's identifier.
    pub fn identifier(&self) -> Option<&Identifier> {
        self.identifier.as_ref()
    }

    /// Replaces the actual identifier by the identifier given in parameter,
    /// returning the old identifier if present.
    pub fn replace_identifier(&mut self, id: Identifier) -> Option<Identifier> {
        self.identifier.replace(id)
    }
}

impl AffineTransform {
    ///  [1 0 0 1 0 0]; the identity transformation.
    fn identity() -> Self {
        AffineTransform {
            x_scale: 1.0,
            xy_scale: 0.,
            yx_scale: 0.,
            y_scale: 1.0,
            x_offset: 0.,
            y_offset: 0.,
        }
    }
}

//NOTE: this is hacky, and intended mostly as a placeholder. It was adapted from
// https://github.com/unified-font-object/ufoLib/blob/master/Lib/ufoLib/filenames.py
/// given a glyph name, compute an appropriate file name.
pub(crate) fn default_file_name_for_glyph_name(name: impl AsRef<str>) -> String {
    fn fn_impl(name: &str) -> String {
        static SPECIAL_ILLEGAL: &[char] = &['\\', '*', '+', '/', ':', '<', '>', '?', '[', ']', '|'];
        static SUFFIX: &str = ".glif";
        const MAX_LEN: usize = 255;

        let mut result = String::with_capacity(name.len());

        for c in name.chars() {
            match c {
                '.' if result.is_empty() => result.push('_'),
                c if (c as u32) < 32 || (c as u32) == 0x7f || SPECIAL_ILLEGAL.contains(&c) => {
                    result.push('_')
                }
                c if c.is_ascii_uppercase() => {
                    result.push(c);
                    result.push('_');
                }
                c => result.push(c),
            }
        }

        //TODO: check for illegal names?
        if result.len() + SUFFIX.len() > MAX_LEN {
            let mut boundary = 255 - SUFFIX.len();
            while !result.is_char_boundary(boundary) {
                boundary -= 1;
            }
            result.truncate(boundary);
        }
        result.push_str(SUFFIX);
        result
    }

    let name = name.as_ref();
    fn_impl(name)
}

impl std::default::Default for AffineTransform {
    fn default() -> Self {
        Self::identity()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Image {
    /// Not an absolute / relative path, but the name of the image file.
    pub file_name: PathBuf,
    pub color: Option<Color>,
    pub transform: AffineTransform,
}

#[cfg(feature = "druid")]
impl From<AffineTransform> for druid::kurbo::Affine {
    fn from(src: AffineTransform) -> druid::kurbo::Affine {
        druid::kurbo::Affine::new([
            src.x_scale as f64,
            src.xy_scale as f64,
            src.yx_scale as f64,
            src.y_scale as f64,
            src.x_offset as f64,
            src.y_offset as f64,
        ])
    }
}

#[cfg(feature = "druid")]
impl From<druid::kurbo::Affine> for AffineTransform {
    fn from(src: druid::kurbo::Affine) -> AffineTransform {
        let coeffs = src.as_coeffs();
        AffineTransform {
            x_scale: coeffs[0] as f32,
            xy_scale: coeffs[1] as f32,
            yx_scale: coeffs[2] as f32,
            y_scale: coeffs[3] as f32,
            x_offset: coeffs[4] as f32,
            y_offset: coeffs[5] as f32,
        }
    }
}

#[cfg(feature = "druid")]
impl From<druid::piet::Color> for Color {
    fn from(src: druid::piet::Color) -> Color {
        let rgba = src.as_rgba_u32();
        let r = ((rgba >> 24) & 0xff) as f32 / 255.0;
        let g = ((rgba >> 16) & 0xff) as f32 / 255.0;
        let b = ((rgba >> 8) & 0xff) as f32 / 255.0;
        let a = (rgba & 0xff) as f32 / 255.0;
        assert!(b >= 0.0 && b <= 1.0, "b: {}, raw {}", b, (rgba & (0xff << 8)));

        Color {
            red: r.max(0.0).min(1.0),
            green: g.max(0.0).min(1.0),
            blue: b.max(0.0).min(1.0),
            alpha: a.max(0.0).min(1.0),
        }
    }
}

#[cfg(feature = "druid")]
impl From<Color> for druid::piet::Color {
    fn from(src: Color) -> druid::piet::Color {
        druid::piet::Color::rgba(src.red, src.green, src.blue, src.alpha)
    }
}
