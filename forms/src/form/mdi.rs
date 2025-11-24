use super::*;

impl MdiClient {
    pub fn activate_child(&self, child: &Form) {
        let Some(parent) = child.rc.control.parent() else {
            warn!("cannot activate MDI child; child form is a top-level form (not an MDI child)");
            return;
        };

        if parent.handle() != self.control.handle() {
            warn!("child form is not parented to this MDI client");
            return;
        }

        unsafe {
            _ = SendMessageW(
                self.control.handle(),
                WM_MDIACTIVATE,
                Some(WPARAM(child.handle().0 as usize)),
                None,
            );
        }
    }
}
