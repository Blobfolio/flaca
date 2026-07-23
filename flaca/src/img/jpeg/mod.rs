/*!
# Flaca: JPEG.
*/

mod jpegtran;
mod strip;

pub(super) use jpegtran::optimize;
pub(super) use strip::JpegMarker;
