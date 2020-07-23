pub const JCOPYOPT_NONE: JCOPY_OPTION = 0;
pub const JCROP_UNSET: JCROP_CODE = 0;
pub const JXFORM_NONE: JXFORM_CODE = 0;
pub type j_common_ptr = *mut jpeg_common_struct;
pub type j_compress_ptr = *mut jpeg_compress_struct;
pub type j_decompress_ptr = *mut jpeg_decompress_struct;
pub type JCOPY_OPTION = u32;
pub type JCROP_CODE = u32;
pub type jvirt_barray_ptr = *mut jvirt_barray_control;
pub type jvirt_sarray_ptr = *mut jvirt_sarray_control;
pub type JXFORM_CODE = u32;

#[repr(C)]
pub struct jpeg_transform_info {
	pub transform: JXFORM_CODE,
	pub perfect: boolean,
	pub trim: boolean,
	pub force_grayscale: boolean,
	pub crop: boolean,
	pub slow_hflip: boolean,
	pub crop_width: JDIMENSION,
	pub crop_width_set: JCROP_CODE,
	pub crop_height: JDIMENSION,
	pub crop_height_set: JCROP_CODE,
	pub crop_xoffset: JDIMENSION,
	pub crop_xoffset_set: JCROP_CODE,
	pub crop_yoffset: JDIMENSION,
	pub crop_yoffset_set: JCROP_CODE,
	pub num_components: ::std::os::raw::c_int,
	pub workspace_coef_arrays: *mut jvirt_barray_ptr,
	pub output_width: JDIMENSION,
	pub output_height: JDIMENSION,
	pub x_crop_offset: JDIMENSION,
	pub y_crop_offset: JDIMENSION,
	pub iMCU_sample_width: ::std::os::raw::c_int,
	pub iMCU_sample_height: ::std::os::raw::c_int,
}

extern "C" {
	pub fn jcopy_markers_setup(srcinfo: j_decompress_ptr, option: JCOPY_OPTION);
}
extern "C" {
	pub fn jcopy_markers_execute(
		srcinfo: j_decompress_ptr,
		dstinfo: j_compress_ptr,
		option: JCOPY_OPTION,
	);
}
extern "C" {
	pub fn jtransform_adjust_parameters(
		srcinfo: j_decompress_ptr,
		dstinfo: j_compress_ptr,
		src_coef_arrays: *mut jvirt_barray_ptr,
		info: *mut jpeg_transform_info,
	) -> *mut jvirt_barray_ptr;
}
extern "C" {
	pub fn jtransform_execute_transform(
		srcinfo: j_decompress_ptr,
		dstinfo: j_compress_ptr,
		src_coef_arrays: *mut jvirt_barray_ptr,
		info: *mut jpeg_transform_info,
	);
}
extern "C" {
	pub fn jtransform_request_workspace(
		srcinfo: j_decompress_ptr,
		info: *mut jpeg_transform_info,
	) -> boolean;
}
