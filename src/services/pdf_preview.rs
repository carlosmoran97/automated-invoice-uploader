use core_graphics::{
    base::{kCGBitmapByteOrder32Little, kCGImageAlphaPremultipliedFirst},
    color_space::CGColorSpace,
    context::CGContext,
    data_provider::CGDataProvider,
    geometry::{CGAffineTransform, CGPoint, CGRect, CGSize},
    sys::{CGContextRef, CGDataProviderRef},
};
use foreign_types::ForeignType;
use image::RgbaImage;
use std::ptr::NonNull;

const CROP_BOX: i32 = 1;
const MAX_PAGE_EDGE: f64 = 2048.;

enum CGPdfDocument {}
enum CGPdfPage {}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPDFDocumentCreateWithProvider(provider: CGDataProviderRef) -> *mut CGPdfDocument;
    fn CGPDFDocumentRelease(document: *mut CGPdfDocument);
    fn CGPDFDocumentGetNumberOfPages(document: *mut CGPdfDocument) -> usize;
    fn CGPDFDocumentGetPage(document: *mut CGPdfDocument, number: usize) -> *mut CGPdfPage;
    fn CGPDFPageGetBoxRect(page: *mut CGPdfPage, pdf_box: i32) -> CGRect;
    fn CGPDFPageGetRotationAngle(page: *mut CGPdfPage) -> i32;
    fn CGPDFPageGetDrawingTransform(
        page: *mut CGPdfPage,
        pdf_box: i32,
        rect: CGRect,
        rotate: i32,
        preserve_aspect_ratio: bool,
    ) -> CGAffineTransform;
    fn CGContextDrawPDFPage(context: CGContextRef, page: *mut CGPdfPage);
}

struct PdfDocument(NonNull<CGPdfDocument>);

impl Drop for PdfDocument {
    fn drop(&mut self) {
        unsafe { CGPDFDocumentRelease(self.0.as_ptr()) }
    }
}

pub(crate) struct RasterizedPage {
    pub page_count: usize,
    // GPUI expects BGRA bytes in its image frame buffer.
    pub bgra: RgbaImage,
}

pub(crate) fn render_page(bytes: &[u8], number: usize) -> Option<RasterizedPage> {
    if bytes.is_empty() || number == 0 {
        return None;
    }
    // The document and its borrowed page are released before the provider and input bytes.
    let provider = unsafe { CGDataProvider::from_slice(bytes) };
    let document = PdfDocument(NonNull::new(unsafe {
        CGPDFDocumentCreateWithProvider(provider.as_ptr())
    })?);
    let page_count = unsafe { CGPDFDocumentGetNumberOfPages(document.0.as_ptr()) };
    if number > page_count {
        return None;
    }
    let page = NonNull::new(unsafe { CGPDFDocumentGetPage(document.0.as_ptr(), number) })?.as_ptr();
    let rect = unsafe { CGPDFPageGetBoxRect(page, CROP_BOX) };
    let (mut width, mut height) = (rect.size.width, rect.size.height);
    if !width.is_finite() || !height.is_finite() || width <= 0. || height <= 0. {
        return None;
    }
    if unsafe { CGPDFPageGetRotationAngle(page) }.rem_euclid(180) == 90 {
        std::mem::swap(&mut width, &mut height);
    }
    let scale = MAX_PAGE_EDGE / width.max(height);
    if !scale.is_finite() {
        return None;
    }
    let page_bounds = CGRect::new(&CGPoint::new(0., 0.), &CGSize::new(width, height));
    let width = (width * scale).round().max(1.) as usize;
    let height = (height * scale).round().max(1.) as usize;
    let bounds = CGRect::new(
        &CGPoint::new(0., 0.),
        &CGSize::new(width as f64, height as f64),
    );
    let mut bitmap = CGContext::create_bitmap_context(
        None,
        width,
        height,
        8,
        width * 4,
        &CGColorSpace::create_device_rgb(),
        kCGImageAlphaPremultipliedFirst | kCGBitmapByteOrder32Little,
    );
    bitmap.set_rgb_fill_color(1., 1., 1., 1.);
    bitmap.fill_rect(bounds);
    // The PDF drawing transform can shrink a page but cannot enlarge it.
    bitmap.scale(scale, scale);
    bitmap
        .concat_ctm(unsafe { CGPDFPageGetDrawingTransform(page, CROP_BOX, page_bounds, 0, true) });
    unsafe { CGContextDrawPDFPage(bitmap.as_ptr(), page) };
    let bgra = RgbaImage::from_raw(width as u32, height as u32, bitmap.data().to_vec())?;
    Some(RasterizedPage { page_count, bgra })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write;

    fn two_page_pdf() -> String {
        let mut pdf = String::from("%PDF-1.4\n");
        let mut offsets = Vec::new();
        let top_left_red = "1 0 0 rg 0 100 50 100 re f\n";
        let stream = format!(
            "<< /Length {} >>\nstream\n{top_left_red}endstream",
            top_left_red.len()
        );
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>",
            "<< /Type /Pages /Kids [3 0 R 4 0 R] /Count 2 /MediaBox [0 0 100 200] /Resources << >> >>",
            "<< /Type /Page /Parent 2 0 R /Contents 5 0 R >>",
            "<< /Type /Page /Parent 2 0 R /Rotate 90 /Contents 5 0 R >>",
            &stream,
        ];
        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            writeln!(pdf, "{} 0 obj\n{object}\nendobj", index + 1).unwrap();
        }
        let xref = pdf.len();
        let size = offsets.len() + 1;
        writeln!(pdf, "xref\n0 {size}\n0000000000 65535 f ").unwrap();
        for offset in offsets {
            writeln!(pdf, "{offset:010} 00000 n ").unwrap();
        }
        writeln!(
            pdf,
            "trailer\n<< /Size {size} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF"
        )
        .unwrap();
        pdf
    }

    #[test]
    fn renders_pages_upright_with_their_pdf_rotation_and_colors() {
        let pdf = two_page_pdf();
        let first = render_page(pdf.as_bytes(), 1).unwrap();
        assert_eq!(first.page_count, 2);
        assert_eq!(first.bgra.dimensions(), (1024, 2048));
        assert_eq!(first.bgra.get_pixel(256, 512).0, [0, 0, 255, 255]);
        assert_eq!(first.bgra.get_pixel(256, 1536).0, [255; 4]);

        let second = render_page(pdf.as_bytes(), 2).unwrap();
        assert_eq!(second.bgra.dimensions(), (2048, 1024));
        assert_eq!(second.bgra.get_pixel(1536, 256).0, [0, 0, 255, 255]);
        assert!(render_page(pdf.as_bytes(), 3).is_none());
    }

    #[test]
    fn rejects_an_attachment_that_is_not_a_pdf() {
        assert!(render_page(b"not a PDF", 1).is_none());
    }
}
