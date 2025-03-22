use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use opencv::{imgproc::cvt_color, prelude::*};

pub fn imagebuffer_to_mat(img: &image::ImageBuffer<image::Rgb<u8>, Vec<u8>>) -> Result<Mat> {
    let width = img.width() as i32;
    let height = img.height() as i32;
    let data = img.as_raw().as_ptr();

    let mut img = unsafe {
        opencv::core::Mat::new_rows_cols_with_data_unsafe(
            height,
            width,
            opencv::core::CV_8UC3,
            data as *mut std::ffi::c_void,
            opencv::core::Mat_AUTO_STEP,
        )
    }?;

    Ok(img)
}

pub fn mat_to_imagebuffer(
    buffer: &mut image::ImageBuffer<image::Rgb<u8>, Vec<u8>>,
    img: &Mat,
) -> Result<()> {
    /// first, check if format of Mat matches, and convert if not
    let bytes = img.data_bytes().unwrap();

    let img_size = bytes.len();
    let buffer_size = buffer.len();

    if img_size != buffer_size {
        // debug!("Size mismatch: {} != {}", img_size, buffer_size);
        // bail!("Size mismatch: {} != {}", size, buffer_size);
        // let mut img2 = img.clone();

        let mut img2 = Mat::new_rows_cols_with_default(
            img.rows(),
            img.cols(),
            opencv::core::CV_8UC3,
            0.0f64.into(),
        )?;

        cvt_color(
            &img,
            &mut img2,
            // COLOR_BGR2GRAY,
            // opencv::imgproc::COLOR_RGB2YUV,
            // opencv::imgproc::COLOR_YUV2RGB,
            opencv::imgproc::COLOR_GRAY2RGB,
            0,
            opencv::core::AlgorithmHint::ALGO_HINT_DEFAULT,
        )
        .unwrap();

        buffer.copy_from_slice(img2.data_bytes().unwrap());
    } else {
        // debug!("Size match: {} == {}", img_size, buffer_size);
        buffer.copy_from_slice(bytes);
    }

    Ok(())
}
