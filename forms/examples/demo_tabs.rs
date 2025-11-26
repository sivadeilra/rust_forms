use forms::*;

const IDC_TABS: ControlId = ControlId(1);
const IDC_ADD_TAB: ControlId = ControlId(2);
const IDC_DELETE_TAB: ControlId = ControlId(3);
const IDC_ZAP: ControlId = ControlId(4);

pub fn main() {
    let app = forms::App::new();

    let form = app
        .form_builder()
        .with(|b| {
            b.size(1024, 768);
            b.title("Tab Control Demo");
        })
        .build();

    let tab_control = TabControl::new(&form);

    let hello_tab = tab_control.add_tab(0, "Hello!");
    hello_tab.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().lead_margin(10).tail_margin(10).fixed(50),
        cols: GridAxis::new()
            .lead_margin(10)
            .tail_margin(10)
            .fixed(50)
            .fixed(50)
            .fixed(50)
            .fixed(50),
        items: vec![
            GridItem::control(
                0,
                0,
                &Button::builder(&form, IDC_ZAP)
                    .text("Zap!")
                    .parent(&hello_tab)
                    .build(),
            ),
            GridItem::control(
                0,
                1,
                &Button::builder(&form, IDC_ZAP)
                    .text("Bop!")
                    .parent(&hello_tab)
                    .build(),
            ),
            GridItem::control(
                0,
                2,
                &Button::builder(&form, IDC_ZAP)
                    .text("Pow!")
                    .parent(&hello_tab)
                    .build(),
            ),
            GridItem::control(
                0,
                3,
                &Button::builder(&form, IDC_ZAP)
                    .text("?@#!")
                    .parent(&hello_tab)
                    .build(),
            ),
        ],
    }));

    let world_tab = tab_control.add_tab(1, "World!");
    world_tab.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().fixed(50),
        cols: GridAxis::new().fixed(200),
        items: vec![GridItem::control(
            0,
            0,
            &Button::builder(&form, IDC_ZAP)
                .text("Zap!")
                .parent(&world_tab)
                .build(),
        )],
    }));

    tab_control.sync_visible();

    let add_tab_button = Button::builder(&form, IDC_ADD_TAB).text("Add tab").build();

    let delete_tab_button = Button::builder(&form, IDC_DELETE_TAB)
        .text("Delete tab")
        .build();

    let buttons_layout = Layout::Stack(
        StackLayout::vertical(30)
            .control(&add_tab_button)
            .control(&delete_tab_button),
    );

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().fixed(50).auto().fixed(50),
        cols: GridAxis::new().auto_min(300).fixed(200),
        items: vec![
            GridItem::control(1, 0, &tab_control),
            GridItem::new(1, 1, LayoutItem::Layout(Box::new(buttons_layout))),
        ],
    }));

    let _form_of = form.command_handler(FormData { tab_control });

    form.show_modal();
}

struct FormData {
    tab_control: TabControl,
}

impl FormHandler for FormData {
    fn notify(&mut self, control: ControlId, notify: Notify) {
        match (control, notify) {
            (IDC_TABS, Notify::ButtonClicked) => {}

            (IDC_ADD_TAB, Notify::ButtonClicked) => {
                self.tab_control.add_tab(0, "foo");
            }

            (IDC_DELETE_TAB, Notify::ButtonClicked) => {}
            _ => {}
        }
    }
}
