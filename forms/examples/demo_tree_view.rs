use forms::*;
use std::cell::Cell;

const IDC_ADD_ROOT: ControlId = ControlId(2);
const IDC_ADD_ITEM: ControlId = ControlId(3);
const IDC_DELETE_ITEM: ControlId = ControlId(4);
const IDC_HAS_LINES: ControlId = ControlId(5);
const IDC_CHECKBOXES: ControlId = ControlId(6);

pub fn main() {
    let app = forms::App::new();

    let form = app.form_builder().size(1024, 768).build().with(|f| {
        f.set_title("List View");
    });

    let tv = TreeView::new(
        &form,
        &TreeViewOptions {
            has_lines: true,
            show_lines_at_root: true,
            ..Default::default()
        },
    );

    let next_item: Cell<u32> = Cell::new(1);
    let get_next_item = move || -> u32 {
        let i = next_item.get();
        next_item.set(i + 1);
        i
    };

    let hello = tv.insert_root("Hello!");
    hello.insert_child("Bonjour");
    hello.insert_child("Hola");
    hello.insert_child("Goddag");
    hello.insert_child("Salve");
    let world = tv.insert_root("World!");
    world.insert_child("Monde");

    hello.expand();
    world.expand();

    let has_lines_button = Button::builder(&form, IDC_HAS_LINES)
        .kind(ButtonKind::AutoCheckBox)
        .build()
        .with(|b| b.set_text("Show lines"));

    let add_root = Button::builder(&form, IDC_ADD_ROOT).build().with(|b| {
        b.set_text("Add root");
    });

    let add_item = Button::builder(&form, IDC_ADD_ITEM).build().with(|b| {
        b.set_text("Add item");
        b.set_enabled(false);
    });

    let delete_item = Button::new(&form, IDC_DELETE_ITEM).with(|b| {
        b.set_text("Delete item");
    });

    let checkboxes_button = Button::builder(&form, IDC_CHECKBOXES)
        .kind(ButtonKind::AutoCheckBox)
        .text("Show checkboxes")
        .build();

    let buttons_layout = Layout::Stack(
        StackLayout::vertical(30)
            .control(add_root.clone())
            .control(add_item.clone())
            .control(delete_item.clone())
            .control(has_lines_button.clone())
            .control(checkboxes_button.clone()),
    );

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().fixed(50).auto().fixed(50),
        cols: GridAxis::new().auto_min(300).fixed(200),
        items: vec![
            GridItem::control(1, 0, tv.clone()),
            GridItem::new(1, 1, LayoutItem::Layout(Box::new(buttons_layout))),
        ],
    }));

    {
        let tv = tv.clone();
        form.command_handler(move |control, command| match (control, command) {
            (IDC_ADD_ROOT, Command::ButtonClicked) => {
                let item_name = format!("{}", get_next_item());
                tv.insert_root(&item_name);
            }

            (IDC_DELETE_ITEM, Command::ButtonClicked) => {}
            (IDC_HAS_LINES, Command::ButtonClicked) => {
                tv.set_has_lines(has_lines_button.is_checked());
            }

            (IDC_CHECKBOXES, Command::ButtonClicked) => {
                tv.set_check_boxes(checkboxes_button.is_checked());
            }

            _ => {}
        });
    }

    form.show_modal();
}
