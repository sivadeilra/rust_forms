use forms::*;
use tracing::debug;

const IDC_MODE_DETAILS: ControlId = ControlId(1);
const IDC_MODE_ICONS: ControlId = ControlId(2);
const IDC_ADD_ITEM: ControlId = ControlId(3);
const IDC_DELETE_ITEM: ControlId = ControlId(4);
const IDC_FULL_ROW_SELECT: ControlId = ControlId(5);
const IDC_CHECKBOXES: ControlId = ControlId(6);
// const IDC_GRIDLINES: ControlId = ControlId(7);

pub fn main() {
    let app = forms::App::new();

    let form = app
        .form_builder()
        .with(|b| {
            b.mdi_frame();
            b.size(1024, 768);
        })
        .build()
        .with(|f| {
            f.set_title("MDI App");
        });

    debug!("creating MDI child window");
    let _child = make_child_form(&form);

    while let Some(event) = app.wait_event_mdi(Some(&form)) {
        match event {
            AppEvent::Quit => break,

            AppEvent::Notify {
                control: control_id,
                notify,
            } => {
                debug!(
                    control_id = control_id.0,
                    ?notify,
                    "unrecognized notification"
                );
            }
        }
    }
}

fn make_child_form(parent: &Form) -> Form {
    let form = parent
        .app()
        .form_builder()
        .with(|b| {
            b.mdi_parent(parent);
            b.title("Hello!");
        })
        .build();

    let lv = ListView::new(&form);
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
            .control(mode_details_button.clone())
            .control(mode_icons_button.clone())
            .control(add_item.clone())
            .control(delete_item.clone())
            .control(full_row_select.clone())
            .control(checkboxes_button.clone()),
    );

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().fixed(50).auto().fixed(50),
        cols: GridAxis::new().auto_min(300).fixed(200),
        items: vec![
            GridItem::control(1, 0, lv.clone()),
            GridItem::new(1, 1, LayoutItem::Layout(Box::new(buttons_layout))),
        ],
    }));

    form
}
