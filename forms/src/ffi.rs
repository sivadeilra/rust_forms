#![allow(dead_code)]

//====== WM_NOTIFY codes (NMHDR.code values) ==================================

pub const NM_FIRST: i32 = 0; // generic to all controls
pub const NM_LAST: i32 = -99;

pub const LVN_FIRST: i32 = -100; // listview
pub const LVN_LAST: i32 = -199;

//====== Generic WM_NOTIFY notification codes =================================

pub const NM_OUTOFMEMORY: i32 = NM_FIRST - 1;
pub const NM_CLICK: i32 = NM_FIRST - 2; // uses NMCLICK struct
pub const NM_DBLCLK: i32 = NM_FIRST - 3;
pub const NM_RETURN: i32 = NM_FIRST - 4;
pub const NM_RCLICK: i32 = NM_FIRST - 5; // uses NMCLICK struct
pub const NM_RDBLCLK: i32 = NM_FIRST - 6;
pub const NM_SETFOCUS: i32 = NM_FIRST - 7;
pub const NM_KILLFOCUS: i32 = NM_FIRST - 8;
pub const NM_CUSTOMDRAW: i32 = NM_FIRST - 12;
pub const NM_HOVER: i32 = NM_FIRST - 13;
pub const NM_NCHITTEST: i32 = NM_FIRST - 14; // uses NMMOUSE struct
pub const NM_KEYDOWN: i32 = NM_FIRST - 15; // uses NMKEY struct
pub const NM_RELEASEDCAPTURE: i32 = NM_FIRST - 16;
pub const NM_SETCURSOR: i32 = NM_FIRST - 17; // uses NMMOUSE struct
pub const NM_CHAR: i32 = NM_FIRST - 18; // uses NMCHAR struct
pub const NM_TOOLTIPSCREATED: i32 = NM_FIRST - 19; // notify of when the tooltips window is create
pub const NM_LDOWN: i32 = NM_FIRST - 20;
pub const NM_RDOWN: i32 = NM_FIRST - 21;
pub const NM_THEMECHANGED: i32 = NM_FIRST - 22;

pub const WM_CTLCOLORSTATIC: u32 = 0x0138;

// vsstyle.h, not in winmd apparently
pub const WP_DIALOG: i32 = 29;
