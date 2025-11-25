use super::*;

static REGISTER_CLASS_ONCE: OnceLock<ATOM> = OnceLock::new();

const FORM_CLASS_NAME: &str = "RustForms_Form";

pub fn register_class_lazy() -> ATOM {
    *REGISTER_CLASS_ONCE.get_or_init(|| unsafe {
        let instance = get_instance();

        let mut class_name_wstr = U16CString::from_str(FORM_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PCWSTR::from_raw(class_name_wstr.as_mut_ptr());
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = HBRUSH((COLOR_BTNFACE.0 + 1) as _);
        class_ex.lpfnWndProc = Some(form_wndproc);
        class_ex.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register window class");
        }
        atom
    })
}

static MDI_CHILD_CLASS_ATOM: OnceLock<ATOM> = OnceLock::new();

const MDI_CHILD_CLASS_NAME: &str = "RustForms_MdiChildWindow";

pub fn register_mdi_child_lazy() -> ATOM {
    *MDI_CHILD_CLASS_ATOM.get_or_init(|| unsafe {
        let instance = get_instance();

        let mut class_name_wstr = U16CString::from_str(MDI_CHILD_CLASS_NAME).unwrap();

        let mut class_ex: WNDCLASSEXW = zeroed();
        class_ex.cbSize = size_of::<WNDCLASSEXW>() as u32;
        class_ex.hInstance = instance;
        class_ex.lpszClassName = PCWSTR::from_raw(class_name_wstr.as_mut_ptr());
        class_ex.style = CS_HREDRAW | CS_VREDRAW;
        class_ex.hbrBackground = HBRUSH((COLOR_BTNFACE.0 + 1) as _);
        class_ex.lpfnWndProc = Some(form_wndproc_mdi_child);
        // class_ex.lpfnWndProc = Some(form_wndproc);
        class_ex.hCursor = LoadCursorW(None, IDC_ARROW).unwrap();
        class_ex.cbWndExtra = size_of::<*mut c_void>() as i32;

        let atom = RegisterClassExW(&class_ex);
        if atom == 0 {
            panic!("Failed to register MDI child window class");
        }
        atom
    })
}

extern "system" fn form_wndproc_mdi_child(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            wm::WM_CREATE => {
                let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut CREATESTRUCTW;
                assert!(!create_struct.is_null());

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                debug!(
                    "WM_CREATE for MDI child, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                /*
                let mdi_create_struct: &MDICREATESTRUCTW =
                    &*(create_params as *const MDICREATESTRUCTW);

                let form_state: *const FormState = mdi_create_struct.lParam.0 as *const FormState;

                debug!(?form_state, "the for-reals Form pointer");

                SetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0), form_state as isize);
                */
                return LRESULT(1);
            }

            wm::WM_CLOSE => {
                debug!("MDI child window received WM_CLOSE; ignoring");
                return LRESULT(0);
            }

            _ => {}
        }

        form_wndproc(window, message, wparam, lparam)
    }
}

extern "system" fn form_wndproc(
    window: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match message {
            wm::WM_CREATE => {
                let create_struct: *mut CREATESTRUCTW = lparam.0 as *mut CREATESTRUCTW;
                assert!(!create_struct.is_null());

                debug!(
                    "WM_CREATE, create params = {:?}",
                    (*create_struct).lpCreateParams
                );

                /*

                let create_params = (*create_struct).lpCreateParams;
                assert!(!create_params.is_null());
                // let form_state: &FormState = &*(create_params as *const FormState);

                SetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0), create_params as isize);
                */
                return LRESULT(1);
            }

            _ => {}
        }

        let form_ptr: isize = GetWindowLongPtrW(window, WINDOW_LONG_PTR_INDEX(0));
        if form_ptr == 0 {
            debug!("form_wndproc: lparam is null, msg {:04x}", message);
            return DefWindowProcW(window, message, wparam, lparam);
        }

        let form: &FormState = &*(form_ptr as *const FormState);

        let app: &App = &form.app;

        match message {
            wm::WM_PAINT => {
                debug!("WM_PAINT");
                // ValidateRect(window, std::ptr::null());

                let mut ps: PAINTSTRUCT = core::mem::zeroed();
                let hdc: HDC = BeginPaint(window, &mut ps);

                _ = EndPaint(window, &ps);

                return LRESULT(0);
            }

            wm::WM_CLOSE => {
                if let Some(exit_code) = form.quit_on_close {
                    debug!("WM_CLOSE: posting quit message");
                    post_quit_message(exit_code);
                } else {
                    debug!("WM_CLOSE: not posting quit message");
                }
            }

            wm::WM_DESTROY => {
                debug!("WM_DESTROY");
                return LRESULT(0);
            }

            wm::WM_GETMINMAXINFO => {
                let min_max: *mut MINMAXINFO = lparam.0 as *mut MINMAXINFO;
                min_max.write(MINMAXINFO {
                    ptMinTrackSize: POINT { x: 400, y: 400 },
                    ptMaxTrackSize: POINT { x: 10000, y: 10000 },
                    ..Default::default()
                });
                return LRESULT(0);
            }

            wm::WM_SIZE => {
                let new_width = (lparam.0 & 0xffff) as u32;
                let new_height = ((lparam.0 >> 16) & 0xffff) as u32;
                debug!("WM_SIZE: {} x {}", new_width, new_height);

                if let Some(sb) = form.status_bar.take() {
                    form.status_bar.set(Some(sb.clone()));
                    SendMessageW(sb.handle(), WM_SIZE, None, None);
                }

                form.invalidate_layout();
                form.ensure_layout_valid();

                // TODO: is this now redundant, due to layout?
                if let Some(ref mdi_client) = form.mdi_client {
                    debug!("setting MDI client size to {} x {}", new_width, new_height);
                    _ = SetWindowPos(
                        mdi_client.control.handle(),
                        None,
                        0,
                        0,
                        new_width as i32,
                        new_height as i32,
                        SWP_NOZORDER,
                    );
                }

                // return 0;
            }

            wm::WM_COMMAND => {
                // https://docs.microsoft.com/en-us/windows/win32/menurc/wm-command

                let control = ControlId(wparam_loword(wparam));
                let command = Command(wparam_hiword(wparam) as u32);

                match command.0 {
                    wm::BN_CLICKED => {
                        debug!("BN_CLICKED");
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ButtonClicked,
                        });
                    }

                    wm::EN_CHANGE => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::EditChange,
                        });
                    }

                    wm::BN_SETFOCUS | wm::EN_SETFOCUS => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::SetFocus,
                        });
                    }

                    wm::BN_KILLFOCUS | wm::EN_KILLFOCUS => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::LostFocus,
                        });
                    }

                    _ => {
                        debug!("unrecognized WM_COMMAND code: {:#4x}", command.0);
                    }
                }

                if let Some(handler) = form.command_handler.get() {
                    handler(control, command);
                } else {
                    debug!("WM_COMMAND: no handler is installed");
                }
            }

            // WM_NOTIFY is used by most of the Common Controls to communicate
            // with the app.
            // https://docs.microsoft.com/en-us/windows/win32/controls/wm-notify
            wm::WM_NOTIFY => {
                let nmhdr_ptr: *mut NMHDR = lparam.0 as *mut NMHDR;
                let hwnd_from: HWND = (*nmhdr_ptr).hwndFrom;
                let notify_code = (*nmhdr_ptr).code;
                // let notify = Notify::from_nmhdr(nmhdr_ptr);

                let control: ControlId = ControlId(wparam.0 as u16);

                use windows::Win32::UI::Controls as controls;

                // For some notifications, we need to handle the notification directly.
                match notify_code {
                    TCN_SELCHANGE => {
                        let tab_controls = form.tab_controls.borrow();
                        for weak_tab_control in tab_controls.iter() {
                            if let Some(tab_control) = weak_tab_control.upgrade() {
                                // TODO: check that this is the right tab control
                                tab_control.sync_visible();
                            }
                        }
                    }

                    // Used for ListView, TreeView
                    // https://learn.microsoft.com/en-us/windows/win32/controls/nm-click-list-view
                    controls::NM_CLICK => {
                        let details: &NMITEMACTIVATE = &*(lparam.0 as *const NMITEMACTIVATE);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ItemClick {
                                item: details.iItem,
                                subitem: details.iSubItem,
                            },
                        });
                    }

                    // Used for ListView, TreeView
                    // https://learn.microsoft.com/en-us/windows/win32/controls/nm-dblclk-list-view
                    controls::NM_DBLCLK => {
                        let details: &NMITEMACTIVATE = &*(lparam.0 as *const NMITEMACTIVATE);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ItemDoubleClick {
                                item: details.iItem,
                                subitem: details.iSubItem,
                            },
                        });
                    }

                    controls::LVN_COLUMNCLICK => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ListColumnClick,
                        });
                    }

                    controls::LVN_ITEMCHANGED => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ListItemChanged,
                        });
                    }

                    controls::LVN_ITEMACTIVATE => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::ListItemActivate,
                        });
                    }

                    controls::NM_RETURN => {
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::Return,
                        });
                    }

                    // https://learn.microsoft.com/en-us/windows/win32/controls/tvn-itemchanged
                    controls::TVN_ITEMCHANGED => {
                        let item_change: &NMTVITEMCHANGE = &*(lparam.0 as *const NMTVITEMCHANGE);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::TreeItemChanged,
                        });
                    }

                    // TreeView - TVN_ITEMEXPANDED
                    // https://learn.microsoft.com/en-us/windows/win32/controls/tvn-itemexpanded
                    controls::TVN_ITEMEXPANDED => {
                        let item_change: &NMTREEVIEWW = &*(lparam.0 as *const NMTREEVIEWW);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::TreeItemExpanded,
                        });
                    }

                    // TreeView - TVN_SELCHANGED
                    // https://learn.microsoft.com/en-us/windows/win32/controls/tvn-selchanged
                    controls::TVN_SELCHANGED => {
                        let item_change: &NMTREEVIEWW = &*(lparam.0 as *const NMTREEVIEWW);
                        app.state.push_event(AppEvent::Notify {
                            control,
                            notify: Notify::TreeItemSelectionChanged,
                        });
                    }

                    _ => {}
                }

                /*
                if let Some(handler) = state.notify_handler.get() {
                    handler(&notify);
                } else {
                    debug!("no WM_NOTIFY handler installed");
                }
                */

                return LRESULT(0);
            }

            // https://docs.microsoft.com/en-us/windows/win32/winmsg/wm-sizing
            wm::WM_SIZING => {
                let (min_width, min_height) = form.layout_min_size.get();
                let window_size: &mut RECT = &mut *(lparam.0 as *mut RECT);
                let height = window_size.bottom - window_size.top;

                // TODO: These adjustments are made to the non-client area,
                // not to the client area.
                let mut adjusted_rect = RECT {
                    top: 0,
                    left: 0,
                    right: min_width,
                    bottom: min_height,
                };

                let window_style = WINDOW_STYLE(GetWindowLongW(window, GWL_STYLE) as u32);

                _ = AdjustWindowRect(&mut adjusted_rect, window_style, false);
                let min_width = adjusted_rect.right - adjusted_rect.left;
                let min_height = adjusted_rect.bottom - adjusted_rect.top;

                // If the width is too small, resist!
                let width = window_size.right - window_size.left;
                if width < min_width {
                    match wparam.0 as u32 {
                        WMSZ_RIGHT | WMSZ_TOPRIGHT | WMSZ_BOTTOMRIGHT => {
                            window_size.right = window_size.left + min_width;
                        }
                        WMSZ_LEFT | WMSZ_TOPLEFT | WMSZ_BOTTOMLEFT => {
                            window_size.left = window_size.right - min_width;
                        }
                        _ => {}
                    }
                }

                // If the height is too small, resist!
                if height < min_height {
                    window_size.bottom = window_size.top + min_height;
                    match wparam.0 as u32 {
                        WMSZ_TOP | WMSZ_TOPLEFT | WMSZ_TOPRIGHT => {
                            window_size.top = window_size.bottom - min_height;
                        }
                        WMSZ_BOTTOM | WMSZ_BOTTOMLEFT | WMSZ_BOTTOMRIGHT => {
                            window_size.bottom = window_size.top + min_height;
                        }
                        _ => {}
                    }
                }
            }

            // https://docs.microsoft.com/en-us/windows/win32/controls/wm-ctlcolorstatic
            wm::WM_CTLCOLORSTATIC => {
                let hdc = HDC(wparam.0 as _);
                let brush = form.background_brush.borrow();
                if let Some(brush) = brush.as_ref() {
                    let hbrush = brush.handle();
                    SelectObject(hdc, HGDIOBJ(hbrush.0));
                    SetBkColor(hdc, COLORREF(form.background_color.get().as_u32()));
                    return LRESULT(hbrush.0 as _);
                }

                return LRESULT(0);
            }

            wm::WM_ERASEBKGND => {
                let mut client_rect: RECT = zeroed();
                _ = GetClientRect(window, &mut client_rect);
                let hdc = HDC(wparam.0 as _);
                let brush = form.background_brush.borrow();
                if let Some(brush) = brush.as_ref() {
                    let hbrush = brush.handle();
                    _ = FillRect(hdc, &client_rect, hbrush);
                    return LRESULT(hbrush.0 as _);
                }
            }

            // MDI frame events
            wm::WM_CHILDACTIVATE => {
                debug!("WM_CHILDACTIVATE");
            }

            wm::WM_MDIACTIVATE => {
                debug!("WM_MDIACTIVATE");
            }

            _ => {
                // allow default to run
            }
        }

        match form.mdi_mode {
            MdiMode::Child => DefMDIChildProcW(window, message, wparam, lparam),
            MdiMode::None => DefWindowProcW(window, message, wparam, lparam),
            MdiMode::Frame => DefFrameProcW(window, None, message, wparam, lparam),
        }
    }
}

#[inline(always)]
fn wparam_loword(wp: WPARAM) -> u16 {
    wp.0 as u16
}

#[allow(dead_code)]
#[inline(always)]
fn wparam_hiword(wp: WPARAM) -> u16 {
    (wp.0 >> 16) as u16
}
