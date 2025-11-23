use std::rc::Rc;

use super::*;
use forms::*;

/// A form that contains just a simple text control, used for generic text output.
pub struct TextForm {
    #[allow(dead_code)]
    pub form: Form,

    pub text_edit: Rc<Edit>,
}

impl TextForm {
    pub fn new(parent_form: &Form) -> Self {
        let form = parent_form
            .app()
            .form_builder()
            .with(|b| {
                b.mdi_parent(parent_form);
            })
            .build();

        let text_edit = Edit::new(&form, IDC_TEXT_FORM_EDIT);
        form.set_layout(Layout::Grid(GridLayout {
            cols: GridAxis::new().auto(),
            rows: GridAxis::new().auto(),
            items: vec![GridItem::control(0, 0, &text_edit)],
        }));

        Self { form, text_edit }
    }
}
