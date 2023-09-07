use windows::w;

use super::*;

pub struct Button {
    control: ControlState,
}

impl core::ops::Deref for Button {
    type Target = ControlState;
    fn deref(&self) -> &ControlState {
        &self.control
    }
}

pub struct ButtonBuilder<'a> {
    form: &'a Rc<Form>,
    parent: Option<&'a ControlState>,
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
    pub fn parent(mut self, parent: &'a ControlState) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn build(self) -> Rc<Button> {
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
    pub fn new(form: &Rc<Form>, id: ControlId) -> Rc<Button> {
        Self::builder(form, id).build()
    }

    pub fn builder(form: &Rc<Form>, id: ControlId) -> ButtonBuilder {
        ButtonBuilder {
            form,
            id,
            kind: None,
            text: None,
            parent: None,
        }
    }

    pub(crate) fn build(builder: ButtonBuilder) -> Rc<Button> {
        let form = builder.form;

        unsafe {
            let parent_window = if let Some(parent) = builder.parent {
                parent.handle()
            } else {
                builder.form.handle()
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
                parent_window,
                HMENU(builder.id.0 as _), // hmenu,
                get_instance(),           // hinstance,
                None,
            );

            if hwnd.0 == 0 {
                panic!("failed to create button window");
            }

            let this = Rc::new(Button {
                control: ControlState::new(hwnd),
            });

            this.set_font(&form.style().button_font);

            if let Some(text) = &builder.text {
                this.set_text(text);
            }

            // let hbr = GetSysColorBrush(SYS_COLOR_INDEX(COLOR_BACKGROUND.0 + 1));
            let hbr = CreateSolidBrush(COLORREF(0xe0_ff_00_ff));
            SetClassLongPtrW(hwnd, GCLP_HBRBACKGROUND, hbr.0 as _);

            this
        }
    }

    pub fn set_enabled(&self, value: bool) {
        unsafe {
            EnableWindow(self.control.handle(), value);
        }
    }

    pub fn set_font(&self, font: &Font) {
        unsafe {
            SendMessageW(
                self.control.handle(),
                WM_SETFONT,
                WPARAM(font.hfont.0 as usize),
                LPARAM(1),
            );
        }
    }

    pub fn set_text(&self, text: &str) {
        set_window_text(self.control.handle(), text);
    }

    pub fn is_checked(&self) -> bool {
        unsafe {
            let result = SendMessageW(self.handle(), BM_GETCHECK, WPARAM(0), LPARAM(0));
            result.0 == BST_CHECKED.0 as isize
        }
    }

    pub fn set_checked(&self, value: bool) {
        unsafe {
            SendMessageW(
                self.handle(),
                BM_SETCHECK,
                if value {
                    WPARAM(BST_CHECKED.0 as _)
                } else {
                    WPARAM(BST_UNCHECKED.0 as _)
                },
                LPARAM(0),
            );
        }
    }

    pub fn get_check_state(&self) -> CheckState {
        unsafe {
            let result = SendMessageW(self.handle(), BM_GETCHECK, WPARAM(0), LPARAM(0));
            CheckState::from_bst(DLG_BUTTON_CHECK_STATE(result.0 as _))
        }
    }

    pub fn set_check_state(&self, value: CheckState) {
        unsafe {
            SendMessageW(
                self.handle(),
                BM_SETCHECK,
                WPARAM(value.to_bst().0 as _),
                LPARAM(0),
            );
        }
    }
}
