//! Translates `MSG` to `Msg`

use crate::ControlId;
use crate::Error;
use windows::Win32::Foundation::{GetLastError, HWND, LPARAM, WPARAM};
use windows::Win32::System::SystemServices::{
    MK_CONTROL, MK_LBUTTON, MK_MBUTTON, MK_RBUTTON, MK_SHIFT,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_CLEAR};
use windows::Win32::UI::WindowsAndMessaging::*; // {MSG, GetMessageW, WM_KEYDOWN};

#[derive(Debug)]
pub enum Msg<'a> {
    Paint,
    Click,
    MouseMove { modifiers: ModifierKeys },
    KeyDown { vkey: VIRTUAL_KEY },
    KeyUp { vkey: VIRTUAL_KEY },

    LButtonDown { x: i16, y: i16 },
    LButtonUp { x: i16, y: i16 },

    Unknown(&'a mut MSG),

    // Button control notifications
    ButtonClicked { control: ControlId }, // BN_CLICKED
}

impl<'a> Msg<'a> {
    pub unsafe fn parse(raw: &'a mut MSG) -> Self {
        use windows::Win32::UI::WindowsAndMessaging as wm;

        match raw.message {
            wm::WM_KEYDOWN => Self::KeyDown {
                vkey: VIRTUAL_KEY(raw.wParam.0 as _),
            },

            wm::WM_KEYUP => Self::KeyDown {
                vkey: VIRTUAL_KEY(raw.wParam.0 as _),
            },

            // https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-lbuttonup
            wm::WM_LBUTTONDOWN => Self::LButtonDown {
                x: get_x_lparam(raw.lParam),
                y: get_y_lparam(raw.lParam),
            },

            wm::WM_LBUTTONUP => Self::LButtonUp {
                x: get_x_lparam(raw.lParam),
                y: get_y_lparam(raw.lParam),
            },

            wm::WM_MOUSEMOVE => Self::MouseMove {
                modifiers: ModifierKeys::from_wparam(raw.wParam),
            },

            _ => Self::Unknown(raw),
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct ModifierKeys(pub u16);

impl ModifierKeys {
    pub fn control(&self) -> bool {
        self.0 & MK_CONTROL.0 as u16 != 0
    }

    pub fn shift(&self) -> bool {
        self.0 & MK_SHIFT.0 as u16 != 0
    }

    pub fn lbutton(&self) -> bool {
        self.0 & MK_LBUTTON.0 as u16 != 0
    }

    pub fn mbutton(&self) -> bool {
        self.0 & MK_MBUTTON.0 as u16 != 0
    }

    pub fn rbutton(&self) -> bool {
        self.0 & MK_RBUTTON.0 as u16 != 0
    }

    #[inline(always)]
    pub fn from_wparam(wparam: WPARAM) -> Self {
        Self((wparam.0 & 0xffff) as u16)
    }
}

impl std::fmt::Debug for ModifierKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.control() {
            write!(f, "CTRL ")?;
        }
        if self.shift() {
            write!(f, " SHIFT ")?;
        }
        if self.lbutton() {
            write!(f, " LBUTTON ")?;
        }
        if self.mbutton() {
            write!(f, " MBUTTON ")?;
        }
        if self.rbutton() {
            write!(f, " RBUTTON ")?;
        }
        Ok(())
    }
}

fn get_x_lparam(lparam: LPARAM) -> i16 {
    (lparam.0 & 0xffff) as i16
}

fn get_y_lparam(lparam: LPARAM) -> i16 {
    ((lparam.0 >> 16) & 0xffff) as i16
}

pub struct MessagePump {
    msg: MSG,
}

impl MessagePump {
    pub fn get(&mut self) -> Result<Option<Msg<'_>>, Error> {
        unsafe {
            let status = GetMessageW(&mut self.msg, HWND(0), 0, 0).0;
            if status < 0 {
                Err(Error::Windows(GetLastError()))
            } else if status == 0 {
                Ok(None)
            } else {
                Ok(Some(Msg::parse(&mut self.msg)))
            }
        }
    }
}
