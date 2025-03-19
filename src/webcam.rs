use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use nokhwa::{
    pixel_format::RgbFormat,
    utils::{RequestedFormat, RequestedFormatType},
};

pub struct Webcam {
    camera: nokhwa::Camera,
    handle: egui::TextureHandle,
}

impl Webcam {
    pub fn spawn_thread(
        mut handle: egui::TextureHandle,
        index: usize,
        //
    ) {
        std::thread::spawn(move || {
            let format =
                // RequestedFormat::<RgbFormat>::new(RequestedFormatType::AbsoluteHighestFrameRate);
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
            let mut camera =
                nokhwa::Camera::new(nokhwa::utils::CameraIndex::Index(index as u32), format)
                    .unwrap();

            // let mut buffer = Vec::with_capacity(1024 * 1024 * 3);

            eprintln!("Starting camera loop");
            loop {
                std::thread::sleep(std::time::Duration::from_millis(100));

                // debug!("looping");

                let Ok(frame) = camera.frame() else {
                    eprintln!("Failed to get frame");
                    continue;
                };

                let res = frame.resolution();

                // debug!("Got frame: {}x{}", res.width(), res.height());

                // frame
                //     .decode_image_to_buffer::<RgbFormat>(&mut buffer)
                //     .unwrap();

                let buffer = frame.decode_image::<RgbFormat>().unwrap();

                // debug!("Decoded frame: {}x{}", res.width(), res.height());

                let img = egui::ColorImage::from_rgb(
                    [res.width() as usize, res.height() as usize],
                    // &buffer[..res.width() as usize * res.height() as usize * 3],
                    buffer.as_flat_samples().as_slice(),
                );

                // debug!("Created image: {}x{}", res.width(), res.height());

                handle.set(img, Default::default());

                // debug!("Set image");
            }

            //
        });
    }
}
