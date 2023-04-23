use super::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ColorRef(u32);

impl ColorRef {
    pub const fn from_rgb(r: u8, b: u8, g: u8) -> ColorRef {
        ColorRef((r as u32) | ((g as u32) << 8) | ((b as u32) << 16))
    }

    pub const fn as_u32(self) -> u32 {
        self.0
    }

    /// R is in the low botw
    pub const fn from_u32_bgr(u: u32) -> Self {
        Self(u)
    }

    /// Bits 24..31 are ignored.
    /// Bits 16..23 are R
    /// Bits 8..15 are G
    /// Bits 0..7 are B
    pub const fn from_u32_rgb(u: u32) -> Self {
        let u = ((u >> 16) & 0xff) // R
        | (u & 0xff00) // G
        | ((u & 0xff) << 16);
        Self(u)
    }

    pub fn from_sys_color(c: SysColor) -> Self {
        unsafe { Self(GetSysColor(SYS_COLOR_INDEX(c as i32))) }
    }
}

impl From<ColorRef> for COLORREF {
    fn from(c: ColorRef) -> COLORREF {
        COLORREF(c.as_u32())
    }
}

macro_rules! well_known_colors {
    (
        $($name:ident = $hex:expr,)*
    ) => {
        impl ColorRef {
            $(
                pub const $name: ColorRef = ColorRef::from_u32_rgb($hex);
            )*
        }
    }
}

well_known_colors! {
    BLACK = 0x00_00_00,
    WHITE = 0xff_ff_ff,
    RED = 0xff_00_00,
    GREEN = 0x00_ff_00,
    BLUE = 0x00_00_ff,
}
