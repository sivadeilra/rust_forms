use super::*;

pub fn demo_list_view(parent: &Form) {
    let form = Form::builder()
        .parent(parent)
        .size(1024, 768)
        .text("List View")
        .build();

    let lv = ListView::new(&form);
    lv.set_full_row_select(true);
    lv.add_column(0, 120, "Stuff");
    lv.add_column(1, 120, "More Stuff");
    lv.set_view(Mode::Details);

    form.show_modal_under(Some(parent));
}
