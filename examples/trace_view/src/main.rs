use forms2::*;
use log::{debug, error, trace};
use regex::Regex;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use trace_reader::*;

mod worker;
use worker::*;

struct AppState {
    path: RefCell<Option<String>>,
    worker_state: Arc<Mutex<AppWorkerState>>,
    form: Form,
    results: ListView,
}

struct AppWorkerState {
    trace_file: Option<TraceFile>,
}

fn main() {
    env_logger::init();

    let f = Form::builder()
        .size(1024, 768)
        .quit_on_close()
        .text("Trace Viewer")
        .build();
    f.set_default_edit_font(Font::builder("Verdana", 18).build().ok());
    f.set_default_button_font(Font::builder("Segoe UI", 24).build().ok());

    let lv = ListView::new(&f, &Rect {
        top: 5,
        left: 5,
        bottom: 5 + 700,
        right: 5 + 800
    });
    lv.set_view(Mode::Details);
    lv.set_full_row_select(true);
    lv.set_grid_lines(true);
    lv.set_check_boxes(true);
    lv.set_double_buffer(true);
    lv.add_column(0, 600, "Stuff");
    lv.add_column(1, 80, "More stuff");

    let app: Rc<AppState> = Rc::new(AppState {
        path: Default::default(),
        form: f.clone(),
        worker_state: Arc::new(Mutex::new(AppWorkerState { trace_file: None })),
        results: lv.clone(),
    });

    let edit = TextBox::new(
        &f,
        &Rect {
            top: 10,
            left: 810,
            right: 810 + 200,
            bottom: 10 + 30,
        },
    );
    edit.set_text("cl.exe");

    let query_button = Button::new(
        &f,
        &Rect {
            top: 60,
            left: 810,
            right: 810 + 200,
            bottom: 60 + 30,
        },
    );
    query_button.set_text("Find Processes");

    let (commands_sender, commands_receiver) = mpsc::channel::<WorkerCommand>();

    // Start things off by opening a new trace file.
    let trace_file_path = r"d:\ES.Build.RustTools\direct2d_buildfre.trc";
    commands_sender
        .send(WorkerCommand::OpenTraceFile(trace_file_path.to_string()))
        .unwrap();

    query_button.on_clicked(EventHandler::new({
        let f = f.clone();
        let apps = app.clone();
        let lv = lv.clone();
        let commands = commands_sender.clone();
        move |()| {
            let query_text = edit.get_text();
            match Regex::new(&query_text) {
                Ok(query_regex) => {
                    lv.delete_all_items();
                    commands
                        .send(WorkerCommand::Query {
                            regex: query_regex,
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

    let response_sender = f.register_receiver_func::<WorkerResponse, _>("status stuff", {
        let app = app.clone();
        move |message: WorkerResponse| {
            app.handle_worker_response(message);
        }
    });

    let _worker_joiner = std::thread::spawn(move || {
        worker_thread(commands_receiver, response_sender);
    });

    f.show_window();

    event_loop();
}

use std::sync::mpsc;

impl AppState {
    fn handle_worker_response(self: &Rc<Self>, message: WorkerResponse) {
        match message {
            WorkerResponse::QueryResult { dir, name } => {
                self.results.insert_item(&dir);
            }

            _ => {
                // nyi
                trace!("received worker response: {:?}", message);
            }
        }
    }
}
