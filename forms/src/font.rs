use windows::Win32::Graphics::Gdi::HFONT;

use super::*;

pub struct Font {
    pub(crate) hfont: HFONT,
}

impl Drop for Font {
    fn drop(&mut self) {
        unsafe {
            DeleteObject(self.hfont);
        }
    }
}

impl Font {
    pub fn new(font_family: &str, height: i32) -> Result<Rc<Font>> {
        Self::builder(font_family, height).build()
    }

    pub fn from_logfont(logfont: &LOGFONTW) -> Result<Font> {
        unsafe {
            let hfont = CreateFontIndirectW(logfont);
            if hfont.0 == 0 {
                warn!("failed to create font");
                return Err(Error::Windows(GetLastError()));
            }

            Ok(Font { hfont })
        }
    }

    pub fn builder(face_name: &str, height: i32) -> FontBuilder<'_> {
        FontBuilder {
            height,
            width: 0,
            face_name,
            italic: false,
            underline: false,
            strikeout: false,
            quality: FontQuality::ClearType,
        }
    }
}

pub struct FontBuilder<'a> {
    height: i32,
    width: i32,
    face_name: &'a str,
    italic: bool,
    underline: bool,
    strikeout: bool,
    quality: FontQuality,
}

impl<'a> FontBuilder<'a> {
    pub fn build(&self) -> Result<Rc<Font>> {
        unsafe {
            let face_name: WCString = WCString::from_str_truncate(self.face_name);
            let hfont = CreateFontW(
                self.height,
                self.width,
                0,                        // escapement,
                0,                        // orientation,
                0,                        // weight,
                self.italic as u32,       // italic,
                self.underline as u32,    // underline,
                self.strikeout as u32,    // strikeout,
                0,                        // charset,
                0,                        // outprecision,
                0,                        // clipprecision,
                self.quality.to_native(), // quality,
                0,                        // pitchandfamily,
                PCWSTR::from_raw(face_name.as_ptr()),
            );

            if hfont.0 == 0 {
                warn!("failed to create font");
                return Err(Error::Windows(GetLastError()));
            }

            Ok(Rc::new(Font { hfont }))
        }
    }

    pub fn italic(&mut self) -> &mut Self {
        self.italic = true;
        self
    }

    pub fn underline(&mut self) -> &mut Self {
        self.underline = true;
        self
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum FontQuality {
    AntiAliased,
    ClearType,
    Default,
    Draft,
    NonAntiAliased,
    Proof,
}

impl FontQuality {
    pub(crate) fn to_native(self) -> u32 {
        match self {
            FontQuality::AntiAliased => ANTIALIASED_QUALITY,
            FontQuality::ClearType => CLEARTYPE_QUALITY,
            FontQuality::Default => DEFAULT_QUALITY,
            FontQuality::Draft => DRAFT_QUALITY,
            FontQuality::NonAntiAliased => NONANTIALIASED_QUALITY,
            FontQuality::Proof => PROOF_QUALITY,
        }
        .0 as u32
    }
}
