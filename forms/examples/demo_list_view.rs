use forms::*;

const IDC_MODE_DETAILS: ControlId = ControlId(1);
const IDC_MODE_ICONS: ControlId = ControlId(2);
const IDC_ADD_ITEM: ControlId = ControlId(3);
const IDC_DELETE_ITEM: ControlId = ControlId(4);
const IDC_FULL_ROW_SELECT: ControlId = ControlId(5);
const IDC_CHECKBOXES: ControlId = ControlId(6);
const IDC_GRIDLINES: ControlId = ControlId(7);

pub fn main() {
    let app = forms::App::new();
    let form = app
        .form_builder()
        .with(|b| {
            b.size(1024, 768);
            b.title("List View");
        })
        .build();

    let lv = ListView::new(&form, None);
    lv.add_column(0, 120, "Stuff");
    lv.add_column(1, 120, "More Stuff");
    lv.set_mode(Mode::Details);
    lv.insert_item("Hello!");
    lv.insert_item("World!");

    let mode_details_button = Button::new(&form, IDC_MODE_DETAILS);
    mode_details_button.set_text("Details");

    let mode_icons_button = Button::new(&form, IDC_MODE_ICONS);
    mode_icons_button.set_text("Icons");

    let add_item = Button::new(&form, IDC_ADD_ITEM);
    add_item.set_text("Add Item");

    let delete_item = Button::new(&form, IDC_DELETE_ITEM);
    delete_item.set_text("Delete Item");

    let full_row_select = Button::builder(&form, IDC_FULL_ROW_SELECT)
        .kind(ButtonKind::AutoCheckBox)
        .text("Full row select")
        .build();

    let checkboxes_button = Button::builder(&form, IDC_CHECKBOXES)
        .kind(ButtonKind::AutoCheckBox)
        .text("Checkboxes")
        .build();

    let buttons_layout = Layout::Stack(
        StackLayout::vertical(30)
            .control(&mode_details_button)
            .control(&mode_icons_button)
            .control(&add_item)
            .control(&delete_item)
            .control(&full_row_select)
            .control(&checkboxes_button),
    );

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().fixed(50).auto().fixed(50),
        cols: GridAxis::new().auto_min(300).fixed(200),
        items: vec![
            GridItem::control(1, 0, &lv),
            GridItem::new(1, 1, LayoutItem::Layout(Box::new(buttons_layout))),
        ],
    }));

    let _form_of = form.command_handler(FormData {
        list_view: lv.clone(),
        full_row_select,
        checkboxes_button,
    });

    form.show_modal();
}

struct FormData {
    list_view: ListView,
    full_row_select: Button,
    checkboxes_button: Button,
}

impl FormHandler for FormData {
    fn notify(&mut self, control: ControlId, notify: Notify) {
        match (control, notify) {
            (IDC_MODE_DETAILS, Notify::ButtonClicked) => {
                self.list_view.set_mode(Mode::Details);
            }

            (IDC_MODE_ICONS, Notify::ButtonClicked) => {
                self.list_view.set_mode(Mode::Icon);
            }

            (IDC_ADD_ITEM, Notify::ButtonClicked) => {
                let name = format!("item #{}", self.list_view.items_len());
                self.list_view.insert_item(&name);
            }

            (IDC_DELETE_ITEM, Notify::ButtonClicked) => {
                let selected_items: Vec<usize> = self.list_view.iter_selected_items().collect();
                for selected_item in selected_items {
                    self.list_view.delete_item(selected_item);
                }
            }

            (IDC_FULL_ROW_SELECT, Notify::ButtonClicked) => {
                self.list_view
                    .set_full_row_select(self.full_row_select.is_checked());
            }

            (IDC_CHECKBOXES, Notify::ButtonClicked) => self
                .list_view
                .set_check_boxes(self.checkboxes_button.is_checked()),

            (IDC_GRIDLINES, Notify::ButtonClicked) => {}
            _ => {}
        }
    }
}
