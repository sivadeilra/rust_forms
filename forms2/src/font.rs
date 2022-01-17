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

    pub fn builder<'a>(face_name: &'a str, height: i32) -> FontBuilder<'a> {
        FontBuilder {
            height,
            width: 0,
            face_name,
            italic: false,
            underline: false,
            strikeout: false,
            quality: FontQuality::AntiAliased,
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
                0, // cescapement,
                0, // corientation,
                0, // cweight,
                self.italic as u32, // bitalic,
                self.underline as u32, // bunderline,
                self.strikeout as u32, // bstrikeout,
                0, // icharset,
                0, // ioutprecision,
                0, // iclipprecision,
                self.quality.to_native(), // iquality,
                0, // ipitchandfamily,
                PWSTR(face_name.as_ptr() as *mut _),
            );

            if hfont == 0 {
                trace!("failed to create font");
                return Err(Error::Windows(GetLastError()));
            }

            return Ok(Rc::new(Font { hfont }));
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
    pub(crate) fn to_native(&self) -> u32 {
        match self {
            FontQuality::AntiAliased => ANTIALIASED_QUALITY,
            FontQuality::ClearType => CLEARTYPE_QUALITY,
            FontQuality::Default => DEFAULT_QUALITY,
            FontQuality::Draft => DRAFT_QUALITY,
            FontQuality::NonAntiAliased => NONANTIALIASED_QUALITY,
            FontQuality::Proof => PROOF_QUALITY,
        }
    }
}
