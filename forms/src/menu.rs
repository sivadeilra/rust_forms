use super::*;

pub struct Menu {
    hmenu: HMENU,
}

impl Drop for Menu {
    fn drop(&mut self) {
        unsafe {
            if !self.hmenu.0.is_null() {
                // debug!("destroying HMENU 0x{:x}", self.hmenu);
                _ = DestroyMenu(self.hmenu);
            } else {
                // debug!("Menu::drop: menu has been taken");
            }
        }
    }
}

impl Menu {
    pub(crate) fn extract(mut self) -> HMENU {
        let hmenu = self.hmenu;
        self.hmenu = HMENU(null_mut());
        hmenu
    }

    pub fn create_menu() -> Self {
        unsafe {
            let hmenu = CreateMenu().unwrap();
            Self { hmenu }
        }
    }

    pub fn create_popup_menu() -> Self {
        unsafe {
            let hmenu = CreatePopupMenu().unwrap();
            Self { hmenu }
        }
    }

    pub fn track_popup_menu(&self, form: &Form, x: i32, y: i32) {
        unsafe {
            let flags = TRACK_POPUP_MENU_FLAGS(0);
            if TrackPopupMenu(self.hmenu, flags, x, y, None, form.handle.get(), None).into() {
                debug!("TrackPopupMenu succeeded");
            } else {
                // debug!("TrackPopupMenu failed");
            }
        }
    }

    pub fn append_menu(&mut self, item: MenuItem<'_>) {
        unsafe {
            let mut flags = (if item.enabled {
                MF_ENABLED
            } else {
                MF_DISABLED
            }) | (if item.grayed {
                MF_GRAYED
            } else {
                MENU_ITEM_FLAGS(0)
            }) | (if item.separator {
                MF_SEPARATOR
            } else {
                MENU_ITEM_FLAGS(0)
            }) | (if item.checked {
                MF_CHECKED
            } else {
                MENU_ITEM_FLAGS(0)
            }) | (if item.string.is_some() {
                MF_STRING
            } else {
                MENU_ITEM_FLAGS(0)
            });

            let wstring: WCString;
            let wstring_ptr: *const u16;
            if let Some(s) = item.string {
                wstring = WCString::from_str_truncate(s);
                wstring_ptr = wstring.as_ptr();
            } else {
                // wstring = Default::default();
                wstring_ptr = null();
            };

            let id: usize;
            if let Some(submenu) = item.submenu {
                flags |= MF_POPUP;
                id = submenu.extract().0 as usize;
            } else {
                id = item.id;
            }

            AppendMenuW(self.hmenu, flags, id, PCWSTR::from_raw(wstring_ptr)).unwrap();
        }
    }

    pub fn item_by_index(&self, index: u32) -> SetItem<'_> {
        trace!("Menu::item_by_index: {}", index);
        SetItem {
            menu: self,
            item: index,
            by_position: true,
        }
    }

    pub fn item_by_id(&self, id: u32) -> SetItem<'_> {
        trace!("Menu::item_by_id: {}", id);
        SetItem {
            menu: self,
            item: id,
            by_position: false,
        }
    }
}

pub struct SetItem<'a> {
    menu: &'a Menu,
    item: u32,
    by_position: bool,
}

impl<'a> SetItem<'a> {
    pub(crate) fn get_state(&self) -> MENU_ITEM_STATE {
        unsafe {
            MENU_ITEM_STATE(GetMenuState(
                self.menu.hmenu,
                self.item,
                if self.by_position {
                    MF_BYPOSITION
                } else {
                    MF_BYCOMMAND
                },
            ))
        }
    }

    pub(crate) fn set_state(&self, new_state: MENU_ITEM_STATE) {
        unsafe {
            let mut item: MENUITEMINFOW = zeroed();
            item.cbSize = size_of::<MENUITEMINFOW>() as u32;
            item.fMask = MIIM_STATE;
            item.fState = new_state;
            // trace!("setting menu item state: new_state 0x{:x}", new_state);
            if SetMenuItemInfoW(self.menu.hmenu, self.item, self.by_position, &item).is_ok() {
                // let readback_state = self.get_state();
                // trace!("readback state: 0x{:x}", readback_state);
            } else {
                warn!("SetMenuItemInfoW failed: {:?}", GetLastError());
            }
        }
    }

    fn set_state_bits(&self, mask: MENU_ITEM_STATE, value: MENU_ITEM_STATE) {
        let old_state = self.get_state();
        let new_state = (old_state & !mask) | value;
        // trace!("set_state_bits: old_state 0x{:x} new_state 0x{:x}", old_state, new_state);
        self.set_state(new_state);
    }

    pub fn checked(&self) -> bool {
        (self.get_state() & MFS_CHECKED).0 != 0
    }

    pub fn set_checked(&self, value: bool) {
        self.set_state_bits(
            MFS_CHECKED,
            if value {
                MFS_CHECKED
            } else {
                MENU_ITEM_STATE(0)
            },
        );
    }

    pub fn enabled(&self) -> bool {
        (self.get_state() & MFS_DISABLED).0 == 0
    }

    pub fn set_enabled(&self, value: bool) {
        self.set_state_bits(MFS_DISABLED, if value { MFS_ENABLED } else { MFS_DISABLED });
    }
}

pub struct MenuItem<'a> {
    pub enabled: bool,
    pub grayed: bool,
    pub separator: bool,
    pub checked: bool,
    pub string: Option<&'a str>,
    pub id: usize,
    pub submenu: Option<Menu>,
}

impl<'a> MenuItem<'a> {
    pub fn separator() -> MenuItem<'a> {
        Self {
            separator: true,
            ..Default::default()
        }
    }
}

impl<'a> Default for MenuItem<'a> {
    fn default() -> Self {
        Self {
            enabled: true,
            grayed: false,
            separator: false,
            checked: false,
            string: None,
            id: 0,
            submenu: None,
        }
    }
}
