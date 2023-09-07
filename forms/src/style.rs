use crate::ColorRef;
use crate::Font;
use crate::SysColor;
use std::rc::Rc;
use windows::Win32::Graphics::Gdi::COLOR_WINDOW;

pub struct Style {
    #[allow(dead_code)]
    pub(crate) background_color: StyleColor,
    pub(crate) button_font: Rc<Font>,
    #[allow(dead_code)]
    pub(crate) button_color: StyleColor,
    pub(crate) edit_font: Rc<Font>,
    pub(crate) static_font: Rc<Font>,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            background_color: StyleColor::SysColor(SysColor::Window),
            button_font: Font::new("Segoe UI", 22).unwrap(),
            button_color: StyleColor::SysColor(SysColor::Menu),
            edit_font: Font::new("Times New Roman", 20).unwrap(),
            static_font: Font::new("Arial", 18).unwrap(),
        }
    }
}

pub enum StyleColor {
    ColorRef(ColorRef),
    SysColor(SysColor),
}
