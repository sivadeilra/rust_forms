use forms::*;

const IDC_TABS: ControlId = ControlId(1);
const IDC_ADD_TAB: ControlId = ControlId(2);
const IDC_DELETE_TAB: ControlId = ControlId(3);
const IDC_ZAP: ControlId = ControlId(4);

pub fn main() {
    env_logger::builder().format_timestamp(None).init();

    let form = Form::builder()
        .size(1024, 768)
        .text("Tab Control Demo")
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
                Button::builder(&form, IDC_ZAP)
                    .text("Zap!")
                    .parent(&hello_tab)
                    .build(),
            ),
            GridItem::control(
                0,
                1,
                Button::builder(&form, IDC_ZAP)
                    .text("Bop!")
                    .parent(&hello_tab)
                    .build(),
            ),
            GridItem::control(
                0,
                2,
                Button::builder(&form, IDC_ZAP)
                    .text("Pow!")
                    .parent(&hello_tab)
                    .build(),
            ),
            GridItem::control(
                0,
                3,
                Button::builder(&form, IDC_ZAP)
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
            Button::builder(&form, IDC_ZAP)
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
            .control(add_tab_button.clone())
            .control(delete_tab_button.clone()),
    );

    form.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis::new().fixed(50).auto().fixed(50),
        cols: GridAxis::new().auto_min(300).fixed(200),
        items: vec![
            GridItem::control(1, 0, tab_control.clone()),
            GridItem::new(1, 1, LayoutItem::Layout(Box::new(buttons_layout))),
        ],
    }));

    {
        let tab_control = tab_control.clone();
        form.command_handler(move |control, command| match (control, command) {
            (IDC_TABS, Command::ButtonClicked) => {}
            (IDC_ADD_TAB, Command::ButtonClicked) => {
                tab_control.add_tab(0, "foo");
            }
            (IDC_DELETE_TAB, Command::ButtonClicked) => {}
            _ => {}
        });
    }

    form.notify_handler(move |_notify: &Notify| {
        // ...
    });

    form.show_modal();
}
