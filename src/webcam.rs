use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use nokhwa::{
    pixel_format::RgbFormat,
    utils::{CameraFormat, RequestedFormat, RequestedFormatType},
};

pub struct Webcam {
    camera: nokhwa::Camera,
    handle: egui::TextureHandle,
}

impl Webcam {
    pub fn spawn_thread(
        ctx: egui::Context,
        mut handle: egui::TextureHandle,
        index: usize,
        crosshair_size: std::sync::Arc<std::sync::atomic::AtomicU32>,
        // channel_to_ui: (),
        //
    ) {
        std::thread::spawn(move || {
            let format =
                RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

            let mut camera =
                nokhwa::Camera::new(nokhwa::utils::CameraIndex::Index(index as u32), format)
                    .unwrap();

            // let backend = camera.backend();
            // debug!("Camera backend: {:?}", backend);

            // let framerate = camera.frame_rate();
            // debug!("Framerate: {}", framerate);

            // camera.set_frame_rate(30).unwrap();

            // let formats = camera.compatible_camera_formats().unwrap();
            // for format in formats {
            //     debug!("Format: {:?}", format);
            // }

            // let ress = camera
            //     .compatible_list_by_resolution(nokhwa::utils::FrameFormat::MJPEG)
            //     .unwrap();
            // for (res, sizes) in ress {
            //     debug!("Resolution: {}x{}", res.width(), res.height());
            // }

            // let controls = camera.supported_camera_controls().unwrap();
            // for control in controls {
            //     debug!("Control: {:?}", control);
            // }

            let format = CameraFormat::new(
                nokhwa::utils::Resolution::new(Self::SIZE.0, Self::SIZE.1),
                nokhwa::utils::FrameFormat::MJPEG,
                30,
            );

            let format = RequestedFormat::new::<RgbFormat>(RequestedFormatType::Closest(format));

            camera.set_camera_requset(format).unwrap();

            // let mut buffer = Vec::with_capacity(Self::SIZE.0 as usize * Self::SIZE.1 as usize * 3);

            let mut buffer: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                image::ImageBuffer::new(Self::SIZE.0 as u32, Self::SIZE.1 as u32);

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

                frame
                    .decode_image_to_buffer::<RgbFormat>(&mut buffer)
                    .unwrap();

                // let buffer = frame.decode_image::<RgbFormat>().unwrap();

                // debug!("Decoded frame: {}x{}", res.width(), res.height());

                let mut img = egui::ColorImage::from_rgb(
                    [res.width() as usize, res.height() as usize],
                    // &buffer[..res.width() as usize * res.height() as usize * 3],
                    buffer.as_flat_samples().as_slice(),
                );

                draw_crosshair(
                    // crosshair_size.load(std::sync::atomic::Ordering::Relaxed) as f32,
                    60., &mut img,
                );

                // debug!("Created image: {}x{}", res.width(), res.height());

                handle.set(img, Default::default());

                ctx.request_repaint();

                // debug!("Set image");
            }

            //
        });
    }
}
