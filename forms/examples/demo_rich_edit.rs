use std::rc::Rc;

use forms::*;

control_ids! {
    IDC_EDIT,
}

fn main() {
    let app = App::new();
    let form = app.form_builder().build().with(|b| {
        b.set_title("Rich Edit");
    });

    let rich_edit = RichEdit::new(&form, IDC_EDIT).with(|b| {
        b.set_auto_vscroll(true);
        b.set_no_hide_selection(true);
        b.set_want_return(true);
    });

    form.set_layout(Layout::Control(Rc::clone(&rich_edit)));

    while let Some(event) = app.wait_event() {
        match event {
            AppEvent::Quit => break,

            AppEvent::Notify { .. } => {}
        }
    }
}
