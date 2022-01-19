/// Initializes a new GUI thread.
pub fn new_gui() -> Gui {
    Gui {}
}

pub struct Gui {}

impl Gui {
    pub fn new_window(&self) -> Window {
        Window {}
    }

    pub fn quit(&self) {}
}

pub struct Window {}

pub struct ListView {}

impl ListView {
    pub fn add_item(&self) {}
}
