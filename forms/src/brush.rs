use super::*;

pub enum Brush {
    System(SystemBrush),
    Owned(OwnedBrush),
}

impl Brush {
    pub(crate) fn handle(&self) -> HBRUSH {
        match self {
            Brush::System(system) => system.0,
            Brush::Owned(owned) => owned.hbrush,
        }
    }

    pub fn from_color_ref(color: ColorRef) -> Result<Brush> {
        unsafe {
            let hbrush = CreateSolidBrush(COLORREF::from(color));
            if !hbrush.0.is_null() {
                Ok(Brush::Owned(OwnedBrush { hbrush }))
            } else {
                Err(Error::Windows(GetLastError()))
            }
        }
    }

    pub fn from_sys_color(color: SysColor) -> Result<Brush> {
        let system_brush = SystemBrush::from_sys_color(color)?;
        Ok(Brush::System(system_brush))
    }
}

pub struct OwnedBrush {
    hbrush: HBRUSH,
}

#[derive(Clone)]
pub struct SystemBrush(HBRUSH);

impl SystemBrush {
    pub fn from_sys_color(color: SysColor) -> Result<SystemBrush> {
        unsafe {
            let hbrush = GetSysColorBrush(SYS_COLOR_INDEX(color as i32));
            if !hbrush.0.is_null() {
                Ok(SystemBrush(hbrush))
            } else {
                Err(Error::Windows(ERROR_NOT_SUPPORTED))
            }
        }
    }
}

// https://docs.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsyscolor
#[repr(i32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum SysColor {
    ActiveBorder = 10,
    ActiveCaption = 2,
    GrayText = 17,
    Highlight = 13,
    HighlightText = 14,
    HotLight = 26,
    Menu = 4,
    WindowFrame = 6,
    Window = 5,
}
