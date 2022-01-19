use forms2::layout::grid::*;
use forms2::*;
use log::{debug, error, info, trace};
use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;
use trace_reader::*;

mod worker;
use worker::*;

struct AppState {
    path: RefCell<Option<String>>,
    form: Form,
    results: Rc<ListView>,
    results_context_menu: Rc<Menu>,
    exec: AsyncExecutor,
}

const IDM_PROPERTIES: u32 = 1;
const IDM_SINGLE_SELECTION: u32 = 2;

fn main() {
    env_logger::init();

    let exec = AsyncExecutor::new();

    let f = Form::builder()
        .size(1024, 768)
        .quit_on_close()
        .text("Trace Viewer")
        .build();
    f.set_default_edit_font(Font::builder("Verdana", 18).build().ok());
    f.set_default_button_font(Font::builder("Segoe UI", 24).build().ok());

    let mut main_menu = Menu::create_menu();
    main_menu.append_menu(MenuItem {
        string: Some("File"),
        submenu: Some({
            let mut file_menu = Menu::create_popup_menu();
            file_menu.append_menu(MenuItem {
                string: Some("&Open"),
                ..Default::default()
            });
            file_menu.append_menu(MenuItem {
                string: Some("Exit"),
                ..Default::default()
            });
            file_menu
        }),
        ..Default::default()
    });
    f.set_menu(Some(main_menu));

    let app: Rc<AppState> = Rc::new(AppState {
        path: Default::default(),
        form: f.clone(),
        results: ListView::new(
            &f,
            &Rect {
                top: 5,
                left: 5,
                bottom: 5 + 700,
                right: 5 + 800,
            },
        ),
        exec: exec.clone(),

        results_context_menu: {
            let mut m = Menu::create_popup_menu();
            m.append_menu(MenuItem {
                string: Some("Hello!"),
                checked: true,
                ..Default::default()
            });
            m.append_menu(MenuItem::separator());
            m.append_menu(MenuItem {
                string: Some("Properties"),
                id: IDM_PROPERTIES as usize,
                ..Default::default()
            });
            m.append_menu(MenuItem {
                string: Some("Single Item"),
                id: IDM_SINGLE_SELECTION as usize,
                ..Default::default()
            });
            Rc::new(m)
        },
    });

    let edit = TextBox::new(&f, None);
    edit.set_text("cl.exe");
    edit.set_tab_stop(true);

    let query_button = Button::new(&f, None);
    query_button.set_text("Find Processes");
    query_button.set_tab_stop(true);

    let b1 = Button::new(&f, None);
    b1.set_text("b1");
    b1.set_tab_stop(true);
    let b2 = Button::new(&f, None);
    b2.set_text("b2");
    b2.set_tab_stop(true);
    let b3 = Button::new(&f, None);
    b3.set_text("b3");
    let b4 = Button::new(&f, None);
    b4.set_text("b4");
    let b5 = Button::new(&f, None);
    b5.set_text("b5");

    let label1 = Label::new(&f, None);
    label1.set_text("Stuff:");

    f.set_layout(Layout::Grid(GridLayout {
        cols: GridAxis {
            padding: 15,
            lead_margin: 10,
            tail_margin: 10,
            cells: vec![
                GridAxisCell::scaled(1.0, 600),
                GridAxisCell::scaled(0.5, 100),
                // GridAxisCell::fixed(100), // labels
                GridAxisCell::fixed(180), // buttons
            ],
        },
        rows: GridAxis {
            padding: 15,
            lead_margin: 10,
            tail_margin: 10,
            cells: vec![
                GridAxisCell::fixed(20),
                GridAxisCell::auto(400),
                GridAxisCell::fixed(30),
            ],
        },
        items: vec![
            GridItem::new(1, 0, LayoutItem::Control(app.results.clone())),
            GridItem {
                row: 1,
                row_span: 2,
                col: 1,
                col_span: 1,
                item: LayoutItem::Layout(Box::new(Layout::Stack(StackLayout {
                    lead_margin: 0,
                    tail_margin: 0,
                    pitch: 30,
                    padding: 4,
                    orientation: Orientation::Vertical,
                    items: vec![LayoutItem::Control(label1)],
                }))),
            },
            GridItem {
                row: 1,
                row_span: 2,
                col: 2,
                col_span: 1,
                item: LayoutItem::Layout(Box::new(Layout::Stack(StackLayout {
                    lead_margin: 0,
                    tail_margin: 0,
                    pitch: 30,
                    padding: 4,
                    orientation: Orientation::Vertical,
                    items: vec![
                        LayoutItem::Control(edit.clone()),
                        LayoutItem::Control(query_button.clone()),
                        LayoutItem::Control(b1.clone()),
                        LayoutItem::Control(b2.clone()),
                        LayoutItem::Control(b3.clone()),
                        LayoutItem::Control(b4.clone()),
                        LayoutItem::Control(b5.clone()),
                    ],
                }))),
            },
        ],
    }));

    app.results.set_view(Mode::Details);
    app.results.set_full_row_select(true);
    app.results.set_grid_lines(true);
    app.results.set_check_boxes(true);
    app.results.set_double_buffer(true);
    app.results.add_column(0, 600, "Stuff");
    app.results.add_column(1, 80, "More stuff");

    app.results.on_rclick(EventHandler::new({
        let app = app.clone();
        move |click: list_view::ItemActivate| {
            info!(
                "rclick: item {}, subitem {}, point ({}, {})",
                click.item, click.subitem, click.point.x, click.point.y
            );

            let mut num_selected: u32 = 0;
            for _i in app.results.iter_selected_items() {
                num_selected += 1;
                if num_selected == 2 {
                    break;
                }
            }

            // Properties is enabled when any items are selected
            app.results_context_menu
                .item_by_id(IDM_PROPERTIES)
                .set_enabled(num_selected > 0);
            app.results_context_menu
                .item_by_id(IDM_SINGLE_SELECTION)
                .set_enabled(num_selected == 1);

            // also test context menus
            let screen_point = app.results.client_to_screen(click.point);
            app.results_context_menu
                .track_popup_menu(&app.form, screen_point.x, screen_point.y);
        }
    }));
    app.results
        .on_click(EventHandler::new(|click: list_view::ItemActivate| {
            info!(
                "click: item {}, subitem {}, point ({}, {})",
                click.item, click.subitem, click.point.x, click.point.y
            );
        }));

    let (commands_sender, commands_receiver) = mpsc::channel::<WorkerCommand>();

    // Start things off by opening a new trace file.
    let trace_file_path = r"d:\ES.Build.RustTools\direct2d_buildfre.trc";
    commands_sender
        .send(WorkerCommand::OpenTraceFile(trace_file_path.to_string()))
        .unwrap();

    query_button.on_clicked(EventHandler::new({
        let app = app.clone();
        let commands = commands_sender.clone();
        move |()| {
            let query_text = edit.get_text();
            match Regex::new(&query_text) {
                Ok(regex) => {
                    app.results.delete_all_items();
                    commands
                        .send(WorkerCommand::Query {
                            regex,
                            max_results: 50,
                        })
                        .unwrap();
                }
                Err(e) => {
                    error!("invalid regex: {:?}", e);
                }
            }
        }
    }));

    let response_sender = f.register_receiver_func::<WorkerResponse, _>("worker", {
        let app = app.clone();
        move |message: WorkerResponse| {
            app.handle_worker_response(message);
        }
    });

    let _worker_joiner = std::thread::spawn(move || {
        worker_thread(commands_receiver, response_sender);
    });

    app.results
        .on_selection_changed(EventHandler::new(move |_| {
            info!("selection changed.");
        }));

    f.show_window();

    exec.spawn("hello".into(), async {
        debug!("hello, world!  this is running in async.");
        // let response = reqwest::get("https://google.com").await;
        // debug!("response: {:#?}", response);
    });

    event_loop();
}

use std::sync::mpsc;

impl AppState {
    fn handle_worker_response(self: &Rc<Self>, message: WorkerResponse) {
        match message {
            WorkerResponse::QueryResult { dir, name } => {
                let item = self.results.insert_item(&dir);
                self.results.set_subitem_text(item, 1, &name);
            }

            _ => {
                // nyi
                trace!("received worker response: {:?}", message);
            }
        }
    }
}
