use super::*;
use std::sync::Once;

static INIT_COMMON_CONTROLS: Once = Once::new();

pub fn init_common_controls() {
    debug!("init_common_controls");
    INIT_COMMON_CONTROLS.call_once(|| unsafe {
        debug!("calling InitCommonControls (in once)");
        let mut icc: INITCOMMONCONTROLSEX = zeroed();
        icc.dwSize = size_of::<INITCOMMONCONTROLSEX>() as u32;
        icc.dwICC = ICC_LISTVIEW_CLASSES | ICC_TREEVIEW_CLASSES | ICC_BAR_CLASSES | ICC_TAB_CLASSES;

        let icc_result = InitCommonControlsEx(&icc).ok();
        debug!("icc_result: {:?}", icc_result);

        SetThemeAppProperties(SET_THEME_APP_PROPERTIES_FLAGS(
            STAP_ALLOW_NONCLIENT | STAP_ALLOW_CONTROLS | STAP_ALLOW_WEBCONTENT,
        ));
    });
}
