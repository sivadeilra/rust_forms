use forms::*;
use std::cell::RefCell;
use std::rc::Rc;

struct AppState {
    paths: Vec<String>,
    next_path: usize,
}

fn main() {
    env_logger::init();

    let apps: Rc<RefCell<AppState>> = Rc::new(RefCell::new(AppState {
        paths: vec![
            r"d:\temp".to_string(),
            r"c:\windows\fonts".to_string(),
            r"d:\os\src".to_string(),
        ],
        next_path: 0,
    }));

    let f = Form::builder()
        .size(800, 600)
        .quit_on_close()
        .text("Hello, World")
        .build();

    f.set_default_edit_font(Font::builder("Verdana", 18).build().ok());
    f.set_default_button_font(Font::builder("Segoe UI", 24).build().ok());

    let lv = ListView::new(
        &f,
        &Rect {
            top: 10,
            left: 5,
            right: 500,
            bottom: 600,
        },
    );

    lv.set_view(Mode::Details);
    lv.set_full_row_select(true);
    lv.set_grid_lines(true);
    lv.set_check_boxes(true);
    lv.set_double_buffer(true);
    lv.add_column(0, 200, "Stuff");
    lv.add_column(1, 80, "More stuff");

    lv.insert_item("Hello");
    lv.insert_item("World");

    let edit = TextBox::new(
        &f,
        Some(&Rect {
            top: 10,
            left: 500,
            right: 500 + 200,
            bottom: 10 + 30,
        }),
    );
    edit.set_text("some path");

    let start = Button::new(
        &f,
        Some(&Rect {
            top: 50,
            left: 500,
            right: 500 + 200,
            bottom: 50 + 30,
        }),
    );
    start.set_text("Start Your Engine");

    start.on_clicked(EventHandler::new({
        let f = f.clone();
        let apps = apps.clone();
        let lv = lv.clone();
        move |()| {
            let path_edit = edit.get_text();
            println!("path: {}", path_edit);

            let mut apps = apps.borrow_mut();
            let next_dir = apps.paths[apps.next_path].clone();
            apps.next_path = (apps.next_path + 1) % apps.paths.len();
            lv.delete_all_items();
            load_directory(&f, &lv, &next_dir);
        }
    }));

    f.show_window();

    event_loop();
}

fn load_directory(f: &Form, lv: &Rc<ListView>, path: &str) {
    let path = path.to_string();
    f.run_background(
        move || {
            let mut names: Vec<String> = Vec::new();
            if let Ok(dir) = std::fs::read_dir(&path) {
                for entry in dir {
                    if let Ok(entry) = entry {
                        names.push(entry.file_name().to_string_lossy().to_string());
                    }
                }
            }

            names
        },
        {
            let lv = lv.clone();
            move |names: std::thread::Result<Vec<String>>| {
                if let Ok(names) = names {
                    for name in names.into_iter() {
                        lv.insert_item(&name);
                    }
                }
            }
        },
    );
}
