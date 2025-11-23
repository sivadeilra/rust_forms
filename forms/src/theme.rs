use super::*;

pub(crate) struct ThemeData {
    pub(crate) button_background_brush: HBRUSH,
}

impl ThemeData {
    pub(crate) fn new() -> Self {
        unsafe {
            #[cfg(false)]
            {
                let button_string = U16CString::from_str_truncate("BUTTON");
                let htheme = OpenThemeData(Some(handle), PCWSTR::from_raw(button_string.as_ptr()));
                if htheme.0 != 0 {
                    debug!("ooo, got theme data");

                    const BP_CHECKBOX: i32 = 3;
                    const CBS_CHECKEDNORMAL: i32 = 5;

                    let part = BP_CHECKBOX;
                    if let Ok(color) =
                        GetThemeColor(htheme, part, CBS_CHECKEDNORMAL, THEME_PROPERTY_SYMBOL_ID(0))
                    {
                        debug!("part {}, got theme color: 0x{:x}", part, color.0);
                    } else {
                        warn!("part {}, failed to get theme color", part);
                    }

                    // dbg!(GetThemeSysColor(htheme, COLOR_MENUTEXT as i32));
                } else {
                    warn!("failed to open theme data for window");
                }

                let htheme = GetWindowTheme(handle);

                let mut logfont: LOGFONTW = zeroed();
                match GetThemeSysFont(Some(htheme), TMT_STATUSFONT, &mut logfont) {
                    Ok(f) => {
                        debug!(
                            "GetThemeFont succeeded: {}",
                            U16CString::from_ptr_str(logfont.lfFaceName.as_ptr()).to_string_lossy()
                        );
                        match Font::from_logfont(&logfont) {
                            Ok(f) => {
                                // form_alloc.default_static_font.set(Some(Rc::new(f)));
                            }
                            Err(_e) => {}
                        }
                        // Font::new(font_family, height)
                    }
                    Err(e) => {
                        warn!("GetThemeFont: {}", e);
                    }
                }
            }

            Self {
                button_background_brush: CreateSolidBrush(COLORREF(0xe0_ff_00_ff)),
            }
        }
    }
}
