use forms::layout::grid::*;
use forms::*;
use regex::Regex;
use std::rc::Rc;
use std::sync::mpsc;

mod worker;
use worker::*;

struct AppState {
    results: Rc<ListView>,
    commands_sender: mpsc::Sender<WorkerCommand>,
    query_button: Rc<Button>,
    root_directory: Rc<TextBox>,
    regex: Rc<TextBox>,
    root_directory_label: Rc<Label>,
    regex_label: Rc<Label>,
    messenger: Messenger,
    status_bar: Rc<StatusBar>,
}

fn main() {
    env_logger::init();

    let form = Form::builder()
        .size(1600, 1200)
        .quit_on_close()
        .text("Search in Files")
        .build();
    form.set_default_edit_font(Font::builder("Verdana", 18).build().ok());
    form.set_default_button_font(Font::builder("Segoe UI", 24).build().ok());

    // Create a channel for our worker thread.
    let (commands_sender, commands_receiver) = mpsc::channel::<WorkerCommand>();

    let app: Rc<AppState> = Rc::new(AppState {
        results: ListView::new(&form).with(|w| {
            w.set_view(Mode::Details);
            w.set_full_row_select(true);
            w.set_grid_lines(true);
            w.add_column(0, 300, "File");
            w.add_column(1, 500, "Matching line");
        }),

        commands_sender,
        query_button: Button::new(&form).with(|w| {
            w.set_text("Search");
            w.set_tab_stop(true);
        }),
        root_directory: TextBox::new(&form).with(|w| {
            w.set_text(r"d:\rust_forms\examples");
        }),
        regex: TextBox::new(&form).with(|w| {
            w.set_text("fn");
        }),
        root_directory_label: Label::new(&form).with(|w| {
            w.set_text("Root dir:");
        }),
        regex_label: Label::new(&form).with(|w| {
            w.set_text("Regex:");
        }),
        messenger: Messenger::new(),
        status_bar: form.create_status_bar(),
    });

    // Set up layout.

    form.set_layout(Layout::Grid(GridLayout {
        cols: GridAxis {
            padding: 15,
            lead_margin: 10,
            tail_margin: 10,
            cells: vec![
                GridAxisCell::fixed(100), // labels
                GridAxisCell::scaled(1.0, 600),
                GridAxisCell::fixed(180), // buttons
            ],
        },
        rows: GridAxis {
            padding: 4,
            lead_margin: 10,
            tail_margin: 10,
            cells: vec![
                GridAxisCell::fixed(30), // file path
                GridAxisCell::fixed(30), // regex, query button
                GridAxisCell::auto(400), // results view
            ],
        },
        items: vec![
            GridItem::new(0, 0, LayoutItem::Control(app.root_directory_label.clone())),
            GridItem::new(0, 1, LayoutItem::Control(app.root_directory.clone())),
            GridItem::new(1, 0, LayoutItem::Control(app.regex_label.clone())),
            GridItem::new(1, 1, LayoutItem::Control(app.regex.clone())),
            GridItem::new(1, 2, LayoutItem::Control(app.query_button.clone())),
            GridItem {
                row: 2,
                row_span: 1,
                col: 0,
                col_span: 3,
                item: LayoutItem::Control(app.results.clone()),
            },
        ],
    }));

    // Set up event handlers.
    app.query_button.on_clicked(EventHandler::new({
        let app = app.clone();
        move |()| {
            let root_directory = app.root_directory.get_text();
            let regex_text = app.regex.get_text();
            match Regex::new(&regex_text) {
                Ok(regex) => {
                    app.status_bar.set_status("Running query...");
                    app.results.delete_all_items();
                    app.commands_sender
                        .send(WorkerCommand::Search {
                            root_directory,
                            regex,
                            max_results: 50,
                            recursive: true,
                        })
                        .unwrap();
                }
                Err(e) => {
                    app.status_bar
                        .set_status(&format!("Invalid regex: {:?}", e));
                }
            }
        }
    }));

    // Start our worker thread.

    let response_tx = app.messenger.register_receiver_func("worker", {
        let app = app.clone();
        move |message: WorkerResponse| {
            app.handle_worker_response(message);
        }
    });

    let _worker = std::thread::spawn(move || {
        worker_thread(commands_receiver, response_tx);
    });

    form.show_modal();
}

impl AppState {
    fn handle_worker_response(self: &Rc<Self>, message: WorkerResponse) {
        match message {
            WorkerResponse::SearchDone => {
                self.status_bar.set_status("Done.");
            }

            WorkerResponse::FileError { file_path, error } => {
                let item = self.results.insert_item(&file_path);
                self.results
                    .set_subitem_text(item, 1, &format!("Error: {:?}", error));
            }

            WorkerResponse::MatchResult { file_path, line } => {
                let item = self.results.insert_item(&file_path);
                self.results.set_subitem_text(item, 1, &line);
            }
        }
    }
}
