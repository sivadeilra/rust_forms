use forms::layout::grid::*;
use forms::*;
use regex::Regex;
use std::rc::Rc;
use std::sync::mpsc;

use tracing::debug;

mod worker;
use worker::*;

struct AppState {
    results: ListView,
    commands_sender: mpsc::Sender<WorkerCommand>,
    query_button: Button,
    root_directory: Edit,
    regex: Edit,
    root_directory_label: Label,
    regex_label: Label,
    messenger: Messenger,
    status_bar: StatusBar,
}

const CONTROL_ID_QUERY_BUTTON: ControlId = ControlId(1);
const CONTROL_ID_ROOT_DIRECTORY: ControlId = ControlId(2);
const CONTROL_ID_REGEX: ControlId = ControlId(3);

fn main() {
    let app = forms::App::new();

    let form = app
        .form_builder()
        .with(|b| {
            b.size(1600, 1200);
            b.quit_on_close();
            b.title("Search in Files");
        })
        .build();

    // form.set_default_edit_font(Font::builder("Verdana", 18).build().ok());
    // form.set_default_button_font(Font::builder("Segoe UI", 24).build().ok());

    // Create a channel for our worker thread.
    let (commands_sender, commands_receiver) = mpsc::channel::<WorkerCommand>();

    let app: Rc<AppState> = Rc::new(AppState {
        results: ListView::new(&form, None).with(|w| {
            w.set_mode(Mode::Details);
            w.set_full_row_select(true);
            w.set_grid_lines(true);
            w.add_column(0, 300, "File");
            w.add_column(1, 500, "Matching line");
        }),

        commands_sender,
        query_button: Button::new(&form, CONTROL_ID_QUERY_BUTTON).with(|w| {
            w.set_text("Search");
            w.set_tab_stop(true);
        }),
        root_directory: Edit::new(&form, CONTROL_ID_ROOT_DIRECTORY).with(|w| {
            w.set_text(r"d:\rust_forms\examples");
        }),
        regex: Edit::new(&form, CONTROL_ID_REGEX).with(|w| {
            w.set_text("fn");
        }),
        root_directory_label: Label::new(&form, "Root dir:"),
        regex_label: Label::new(&form, "Regex:"),
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
            GridItem::new(0, 0, LayoutItem::control(&app.root_directory_label)),
            GridItem::new(0, 1, LayoutItem::control(&app.root_directory)),
            GridItem::new(1, 0, LayoutItem::control(&app.regex_label)),
            GridItem::new(1, 1, LayoutItem::control(&app.regex)),
            GridItem::new(1, 2, LayoutItem::control(&app.query_button)),
            GridItem::new(2, 0, LayoutItem::control(&app.results)).col_span(3),
        ],
    }));

    {
        let app = app.clone();
        form.command_handler(move |control, command| match (control, command) {
            (CONTROL_ID_QUERY_BUTTON, Command::ButtonClicked) => {
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
            (id, cmd) => {
                debug!(?id, ?cmd, "command not recognized");
            }
        });
    }

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
