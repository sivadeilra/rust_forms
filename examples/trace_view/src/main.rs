use forms::layout::grid::*;
use forms::Result;
use forms::*;
use log::{debug, error, info, trace};
use regex::Regex;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::mpsc;
use trace_reader::*;

mod worker;
use worker::*;

struct AppState {
    #[allow(dead_code)]
    path: RefCell<Option<String>>,
    form: Rc<Form>,
    processes_list_view: Rc<ListView>,
    process_detail_view: Rc<Edit>,
    process_detail_sequence_number: Cell<u64>,
    results_context_menu: Rc<Menu>,
    scope_tree: Rc<TreeView>,

    results_data: RefCell<HashMap<usize, ProcessItem>>,

    #[allow(dead_code)]
    exec: AsyncExecutor,

    commands_sender: mpsc::Sender<WorkerCommand>,
    query_button: Rc<Button>,

    status: Rc<StatusBar>,

    num_results: Cell<u32>,

    #[allow(dead_code)]
    monospace_font: Rc<Font>,

    messenger: Messenger,
}

struct ProcessItem {
    command_string_offset: StringIndex,
}

const IDM_PROPERTIES: u32 = 1;
const IDM_SINGLE_SELECTION: u32 = 2;

fn main() -> Result<()> {
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

    let (commands_sender, commands_receiver) = mpsc::channel::<WorkerCommand>();

    let monospace_font = Font::builder("Courier New", 16).build().unwrap();

    let app: Rc<AppState> = Rc::new(AppState {
        path: Default::default(),
        form: f.clone(),
        processes_list_view: ListView::new(&f).with(|w| {
            w.set_view(Mode::Details);
            w.set_full_row_select(true);
            w.set_grid_lines(true);
            w.set_check_boxes(true);
            w.set_double_buffer(true);
            w.add_column(0, 450, "Directory");
            w.add_column(1, 350, "Tool");
        }),
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
        process_detail_view: Edit::new_with_options(
            &f,
            EditOptions {
                multiline: true,
                readonly: true,
                vertical_scrollbar: true,
                ..Default::default()
            },
        )
        .with(|w| {
            w.set_font(monospace_font.clone());
        }),
        process_detail_sequence_number: Cell::new(0),
        results_data: Default::default(),
        commands_sender,
        query_button: Button::new(&f).with(|w| {
            w.set_text("Find Processes");
            w.set_tab_stop(true);
        }),
        messenger: Messenger::new(),
        num_results: Cell::new(0),
        scope_tree: TreeView::new(
            &f,
            &TreeViewOptions {
                // has_lines: true,
                full_row_select: true,
                always_show_selection: true,
                has_buttons: true,
                checkboxes: true,
                ..Default::default()
            },
        ),
        status: f.create_status_bar(),
        monospace_font,
    });

    let alpha = app.scope_tree.insert_root("Alpha")?;
    alpha.insert_child("Zap")?;
    alpha.insert_child("Dingbat")?;
    let beta = app.scope_tree.insert_root("Beta")?;
    let beta_courier = beta.insert_child("Courier")?;
    let xyzzy = beta_courier.insert_child("Xyzzy")?;
    xyzzy.set_data("this is some stuff".to_string());
    // beta_courier.expand();
    beta.insert_child("Ring")?;
    // alpha.expand();
    // beta.expand();
    // beta_courier.delete();
    xyzzy.ensure_visible();

    app.scope_tree.on_selection_changed(EventHandler::new({
        let app = app.clone();
        move |selection: tree_view::SelectionChanged| {
            if let Some(sel) = selection.0.as_ref() {
                let data_borrow = sel.data().borrow();
                if let Some(data) = &*data_borrow {
                    if let Some(d) = data.downcast_ref::<String>() {
                        app.status
                            .set_status(&format!("Something got clicked!  {:?}", d));
                    } else {
                        app.status
                            .set_status(&format!("Something got clicked!  {:?}", data));
                    }
                } else {
                    app.status
                        .set_status("Something got clicked!  (node, but no data)");
                }
            } else {
                app.status.set_status("Something got clicked!  (no node)");
            }
        }
    }));

    let edit = Edit::new(&f);
    edit.set_text("cl.exe");
    edit.set_tab_stop(true);

    let close_button = Button::new(&f);
    close_button.set_text("Close File");

    let open_button = Button::new(&f);
    open_button.set_text("Open");
    open_button.set_tab_stop(true);
    open_button.on_clicked(EventHandler::new({
        use forms::file_dialog::*;
        let app = app.clone();
        move |()| {
            let mut fd = FileDialog::new();
            fd.filters = FileFilter::Static(&[("Build traces (*.trc;*.etl)", "*.trc;*.etl")]);
            fd.initial_dir = Some(r"D:\ES.BuildTools.Rust".to_string());
            fd.title = Some("Select build trace file".to_string());
            match fd.open(Some(&app.form)) {
                FileDialogResult::Single(filename) => {
                    app.open_trace_file(&filename);
                }
                _ => {}
            }

            app.messenger.run_background(
                move || {
                    info!("this is a background task");
                    42
                },
                |result: std::thread::Result<u64>| match result {
                    Ok(value) => info!("the background task completed.  on ui thread: {}", value),
                    Err(e) => info!("background task panicked: {:?}", e),
                },
            );
        }
    }));

    let save_button = Button::new(&f);
    save_button.set_text("&Save");
    save_button.set_tab_stop(true);
    save_button.on_clicked(EventHandler::new({
        let app = app.clone();
        move |()| {
            let fd = forms::file_dialog::FileDialog::new();
            // fd.allow_multi_select = true;
            fd.save(Some(&app.form));
        }
    }));

    let search_regex_label = Label::new(&f);
    search_regex_label.set_text("Search (regex):");

    f.set_layout(Layout::Grid(GridLayout {
        rows: GridAxis {
            padding: 15,
            lead_margin: 10,
            tail_margin: 10,
            cells: vec![
                GridAxisCell::fixed(20),
                GridAxisCell::fixed(350),
                GridAxisCell::auto(40),
                // GridAxisCell::fixed(30),
            ],
        },
        cols: GridAxis {
            padding: 15,
            lead_margin: 10,
            tail_margin: 10,
            cells: vec![
                GridAxisCell::scaled(1.0, 600),
                GridAxisCell::fixed(100), // labels
                GridAxisCell::fixed(180), // buttons
            ],
        },
        items: vec![
            GridItem::new(1, 0, LayoutItem::Control(app.processes_list_view.clone())),
            GridItem::new(2, 0, LayoutItem::Control(app.process_detail_view.clone())),
            GridItem {
                row: 2,
                row_span: 1,
                col: 1,
                col_span: 2,
                item: LayoutItem::Control(app.scope_tree.clone()),
            },
            GridItem {
                row: 1,
                row_span: 1,
                col: 1,
                col_span: 1,
                item: LayoutItem::Layout(Box::new(Layout::Stack(StackLayout {
                    lead_margin: 0,
                    tail_margin: 0,
                    pitch: 30,
                    padding: 4,
                    orientation: Orientation::Vertical,
                    items: vec![LayoutItem::Control(search_regex_label)],
                }))),
            },
            GridItem {
                row: 1,
                row_span: 1,
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
                        LayoutItem::Control(app.query_button.clone()),
                        LayoutItem::Control(close_button.clone()),
                        LayoutItem::Control(open_button.clone()),
                        LayoutItem::Control(save_button.clone()),
                    ],
                }))),
            },
        ],
    }));

    app.processes_list_view.on_click(EventHandler::new({
        let app = app.clone();
        move |_| {
            if let Some(first) = app.processes_list_view.iter_selected_items().next() {
                debug!("process key: {}", first);
                let results_data = app.results_data.borrow();
                if let Some(entry) = results_data.get(&first) {
                    debug!("found process info");
                    app.get_process_details(entry.command_string_offset);
                } else {
                    debug!("no process info");
                }
            } else {
                debug!("no item selected");
            }
        }
    }));

    app.processes_list_view.on_rclick(EventHandler::new({
        let app = app.clone();
        move |click: list_view::ItemActivate| {
            info!(
                "rclick: item {}, subitem {}, point ({}, {})",
                click.item, click.subitem, click.point.x, click.point.y
            );

            let mut num_selected: u32 = 0;
            for _i in app.processes_list_view.iter_selected_items() {
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
            let screen_point = app.processes_list_view.client_to_screen(click.point);
            app.results_context_menu
                .track_popup_menu(&app.form, screen_point.x, screen_point.y);
        }
    }));
    app.processes_list_view
        .on_click(EventHandler::new(|click: list_view::ItemActivate| {
            info!(
                "click: item {}, subitem {}, point ({}, {})",
                click.item, click.subitem, click.point.x, click.point.y
            );
        }));

    // Start things off by opening a new trace file.
    app.open_trace_file(r"d:\ES.Build.RustTools\direct2d_buildfre.trc");

    app.query_button.on_clicked(EventHandler::new({
        let app = app.clone();
        move |()| {
            let query_text = edit.get_text();
            match Regex::new(&query_text) {
                Ok(regex) => {
                    app.processes_list_view.delete_all_items();
                    app.status.set_status("Searching...");
                    app.commands_sender
                        .send(WorkerCommand::Query {
                            regex,
                            max_results: 1000,
                        })
                        .unwrap();
                }
                Err(e) => {
                    app.status.set_status(&format!("Invalid regex: {:?}", e));
                }
            }
        }
    }));
    app.query_button.set_enabled(false);

    let response_sender = app
        .messenger
        .register_receiver_func::<WorkerResponse, _>("worker", {
            let app = app.clone();
            move |message: WorkerResponse| {
                app.handle_worker_response(message);
            }
        });

    let _worker_joiner = std::thread::spawn(move || {
        worker_thread(commands_receiver, response_sender);
    });

    close_button.on_clicked(EventHandler::new({
        let app = app.clone();
        move |_| {
            app.close_trace_file();
        }
    }));

    f.show_modal();
    Ok(())
}

impl AppState {
    fn get_process_details(self: &Rc<Self>, command_string_offset: StringIndex) {
        let seqno = self.process_detail_sequence_number.get() + 1;
        self.process_detail_sequence_number.set(seqno);
        let _ = self.commands_sender.send(WorkerCommand::GetProcessDetail {
            sequence_number: seqno,
            command_string_offset,
        });
    }

    fn open_trace_file(&self, trace_file_path: &str) {
        self.commands_sender
            .send(WorkerCommand::OpenTraceFile(trace_file_path.to_string()))
            .unwrap();
        self.status.set_status("Opening trace file...");
    }

    fn close_trace_file(&self) {
        self.query_button.set_enabled(false);
        let _ = self.commands_sender.send(WorkerCommand::CloseTraceFile);
        self.status.set_status("Closed.");
    }

    fn handle_worker_response(self: &Rc<Self>, message: WorkerResponse) {
        match message {
            WorkerResponse::QueryResult {
                dir,
                name,
                command_string_offset,
            } => {
                let item = self.processes_list_view.insert_item(&dir);
                self.processes_list_view.set_subitem_text(item, 1, &name);
                let num_results = self.num_results.get() + 1;
                self.num_results.set(num_results);
                if num_results % 100 == 0 {
                    self.status
                        .set_status(&format!("Found {} results...", num_results));
                }

                let mut results_data = self.results_data.borrow_mut();
                trace!("inserting process data for item {}", item);
                results_data.insert(
                    item,
                    ProcessItem {
                        command_string_offset,
                    },
                );
            }

            WorkerResponse::OpenFailed(e) => {
                error!("failed to open trace file: {:?}", e);
                self.query_button.set_enabled(false);
                self.status
                    .set_status(&format!("Failed to open trace file: {:?}", e));
            }

            WorkerResponse::OpenSucceeded => {
                self.query_button.set_enabled(true);
                self.status.set_status("Opened trace file.");
            }

            WorkerResponse::QueryDone {
                num_records_scanned,
                elapsed,
            } => {
                self.status.set_status(&format!(
                    "Search is finished. Scanned {} records in {:?}",
                    num_records_scanned, elapsed
                ));
            }

            WorkerResponse::ProcessDetail {
                command_string,
                sequence_number,
            } => {
                let current_seqno = self.process_detail_sequence_number.get();
                if sequence_number != current_seqno {
                    debug!("wrong sequence number (too many in-flight queries)");
                } else {
                    let mut wrapped = String::new();
                    for arg in tool_parser::args::iter_args(&command_string) {
                        wrapped.push_str(&arg);
                        wrapped.push_str("\r\n");
                    }
                    self.process_detail_view.set_text(&wrapped);
                }
            }
        }
    }
}
