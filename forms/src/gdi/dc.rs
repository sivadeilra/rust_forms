use windows::Win32::Foundation::{COLORREF, POINT};
use windows::Win32::Graphics::Gdi::HDC;
use windows::Win32::Graphics::Gdi::{self, HGDIOBJ};

use crate::ColorRef;

pub struct Dc {
    pub(crate) hdc: HDC,
}

impl Dc {
    #[inline(always)]
    pub fn begin_path(&self) {
        unsafe {
            Gdi::BeginPath(self.hdc);
        }
    }

    #[inline(always)]
    pub fn end_path(&self) {
        unsafe {
            Gdi::EndPath(self.hdc);
        }
    }

    pub fn close_figure(&self) {
        unsafe {
            Gdi::CloseFigure(self.hdc);
        }
    }

    #[inline(always)]
    pub fn move_to(&self, x: i32, y: i32) {
        unsafe {
            Gdi::MoveToEx(self.hdc, x, y, None);
        }
    }

    #[inline(always)]
    pub fn line_to(&self, x: i32, y: i32) {
        unsafe {
            Gdi::LineTo(self.hdc, x, y);
        }
    }

    #[inline(always)]
    pub fn polyline(&self, points: &[POINT]) {
        unsafe {
            Gdi::Polyline(self.hdc, points);
        }
    }

    #[inline(always)]
    pub fn polyline_to(&self, points: &[POINT]) {
        unsafe {
            Gdi::PolylineTo(self.hdc, points);
        }
    }

    #[inline(always)]
    pub fn poly_bezier(&self, points: &[POINT]) {
        unsafe {
            Gdi::PolyBezier(self.hdc, points);
        }
    }

    #[inline(always)]
    pub fn poly_bezier_to(&self, points: &[POINT]) {
        unsafe {
            Gdi::PolyBezierTo(self.hdc, points);
        }
    }

    #[inline(always)]
    pub fn arc(&self, x1: i32, y1: i32, x2: i32, y2: i32, x3: i32, y3: i32, x4: i32, y4: i32) {
        unsafe {
            Gdi::Arc(self.hdc, x1, y1, x2, y2, x3, y3, x4, y4);
        }
    }

    #[inline(always)]
    pub fn arc_to(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        xr1: i32,
        yr1: i32,
        xr2: i32,
        yr2: i32,
    ) {
        unsafe {
            Gdi::ArcTo(self.hdc, left, top, right, bottom, xr1, yr1, xr2, yr2);
        }
    }

    #[inline(always)]
    pub fn rectangle(&self, left: i32, top: i32, right: i32, bottom: i32) {
        unsafe {
            Gdi::Rectangle(self.hdc, left, top, right, bottom);
        }
    }

    #[inline(always)]
    pub fn round_rect(
        &self,
        left: i32,
        top: i32,
        right: i32,
        bottom: i32,
        width: i32,
        height: i32,
    ) {
        unsafe {
            Gdi::RoundRect(self.hdc, left, top, right, bottom, width, height);
        }
    }

    #[inline(always)]
    pub fn ellipse(&self, left: i32, top: i32, right: i32, bottom: i32) {
        unsafe {
            Gdi::Ellipse(self.hdc, left, top, right, bottom);
        }
    }

    pub fn stroke_path(&self) {
        unsafe {
            Gdi::StrokePath(self.hdc);
        }
    }

    pub fn fill_path(&self) {
        unsafe {
            Gdi::FillPath(self.hdc);
        }
    }

    // https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-selectobject
    pub fn select_object(&self, h: isize) {
        unsafe {
            Gdi::SelectObject(self.hdc, HGDIOBJ(h));
        }
    }

    pub fn set_pen_color(&self, c: ColorRef) {
        unsafe {
            Gdi::SetDCPenColor(self.hdc, COLORREF(c.as_u32()));
        }
    }

    pub fn text_out_w(&self, x: i32, y: i32, chars: &[u16]) {
        unsafe {
            Gdi::TextOutW(self.hdc, x, y, chars);
        }
    }

    pub fn text_out_a(&self, x: i32, y: i32, chars: &[u8]) {
        unsafe {
            Gdi::TextOutA(self.hdc, x, y, chars);
        }
    }

    // pub fn select_brush(&self, pen: HPEN) {
    // }
}
