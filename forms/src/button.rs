use windows::core::w;

use super::*;

#[derive(Clone)]
pub struct Button {
    control: Rc<ControlState>,
}

impl AsRef<Rc<ControlState>> for Button {
    fn as_ref(&self) -> &Rc<ControlState> {
        &self.control
    }
}

impl core::ops::Deref for Button {
    type Target = Rc<ControlState>;
    fn deref(&self) -> &Rc<ControlState> {
        &self.control
    }
}

pub struct ButtonBuilder<'a> {
    form: &'a Form,
    parent: Option<Rc<ControlState>>,
    id: ControlId,
    kind: Option<ButtonKind>,
    text: Option<String>,
}

impl<'a> ButtonBuilder<'a> {
    #[must_use]
    pub fn kind(mut self, kind: ButtonKind) -> Self {
        self.kind = Some(kind);
        self
    }

    #[must_use]
    pub fn text(mut self, text: &str) -> Self {
        self.text = Some(text.to_string());
        self
    }

    #[must_use]
    pub fn parent(mut self, parent: &Rc<ControlState>) -> Self {
        self.parent = Some(parent.clone());
        self
    }

    pub fn build(self) -> Button {
        Button::build(self)
    }
}

#[derive(Copy, Clone, Eq, PartialEq)]
pub enum ButtonKind {
    Command,
    CheckBox,
    AutoCheckBox,
    ThreeState,
    AutoThreeState,
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CheckState {
    Checked,
    Unchecked,
    Indeterminate,
}

impl CheckState {
    fn from_bst(bst: DLG_BUTTON_CHECK_STATE) -> Self {
        match bst {
            BST_CHECKED => Self::Checked,
            BST_UNCHECKED => Self::Unchecked,
            BST_INDETERMINATE => Self::Indeterminate,
            _ => Self::Indeterminate,
        }
    }

    fn to_bst(self) -> DLG_BUTTON_CHECK_STATE {
        match self {
            Self::Checked => BST_CHECKED,
            Self::Unchecked => BST_UNCHECKED,
            Self::Indeterminate => BST_INDETERMINATE,
        }
    }
}

impl Button {
    pub fn new(form: &Form, id: ControlId) -> Button {
        Self::builder(form, id).build()
    }

    pub fn builder(form: &Form, id: ControlId) -> ButtonBuilder<'_> {
        ButtonBuilder {
            form,
            id,
            kind: None,
            text: None,
            parent: None,
        }
    }

    pub(crate) fn build(builder: ButtonBuilder) -> Button {
        let form = builder.form;

        unsafe {
            let parent_window: Rc<ControlState> = if let Some(parent) = builder.parent {
                parent
            } else {
                Rc::clone(&*builder.form)
            };

            let ex_style = 0;

            let mut window_style = WS_CHILD | WS_VISIBLE;

            window_style.0 |= match builder.kind {
                None => BS_DEFPUSHBUTTON,
                Some(ButtonKind::AutoCheckBox) => BS_AUTOCHECKBOX,
                Some(ButtonKind::AutoThreeState) => BS_AUTO3STATE,
                Some(ButtonKind::Command) => BS_DEFPUSHBUTTON,
                Some(ButtonKind::ThreeState) => BS_3STATE,
                Some(ButtonKind::CheckBox) => BS_CHECKBOX,
            } as u32;

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE(ex_style),
                w!("BUTTON"),
                PCWSTR::from_raw(null_mut()),
                window_style,
                0,
                0,
                0,
                0,
                Some(parent_window.handle()),
                Some(HMENU(builder.id.0 as _)), // hmenu,
                Some(get_instance()),           // hinstance,
                None,
            )
            .unwrap();

            let this = Button {
                control: ControlState::new(hwnd, Some(parent_window)),
            };

            this.set_font(&form.style().button_font);

            if let Some(text) = &builder.text {
                this.set_text(text);
            }

            let hbr = builder.form.rc.app.state.theme.button_background_brush;
            SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, hbr.0 as _);

            this
        }
    }

    pub fn set_enabled(&self, value: bool) {
        unsafe {
            _ = EnableWindow(self.control.handle(), value);
        }
    }

    pub fn set_font(&self, font: &Font) {
        unsafe {
            SendMessageW(
                self.control.handle(),
                WM_SETFONT,
                Some(WPARAM(font.hfont.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.control.handle(), text);
    }

    pub fn is_checked(&self) -> bool {
        unsafe {
            let result = SendMessageW(self.handle(), BM_GETCHECK, None, None);
            result.0 == BST_CHECKED.0 as isize
        }
    }

    pub fn set_checked(&self, value: bool) {
        unsafe {
            _ = SendMessageW(
                self.handle(),
                BM_SETCHECK,
                if value {
                    Some(WPARAM(BST_CHECKED.0 as _))
                } else {
                    Some(WPARAM(BST_UNCHECKED.0 as _))
                },
                None,
            );
        }
    }

    pub fn get_check_state(&self) -> CheckState {
        unsafe {
            let result = SendMessageW(self.handle(), BM_GETCHECK, None, None);
            CheckState::from_bst(DLG_BUTTON_CHECK_STATE(result.0 as _))
        }
    }

    pub fn set_check_state(&self, value: CheckState) {
        unsafe {
            _ = SendMessageW(
                self.handle(),
                BM_SETCHECK,
                Some(WPARAM(value.to_bst().0 as _)),
                None,
            );
        }
    }
}
