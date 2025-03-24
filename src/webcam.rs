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
    // pub const SIZE: (u32, u32) = (640, 480);
    // pub const SIZE: (u32, u32) = (320, 240);
    pub const SIZE: (u32, u32) = (1280, 800);

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

pub fn draw_crosshair(radius: f32, img: &mut egui::ColorImage) {
    let width = img.width();
    let height = img.height();

    // Center coordinates
    let center_x = width / 2;
    let center_y = height / 2;

    // Crosshair size (length of each line from center)
    let line_length = width.min(height) / 20;

    // Crosshair color (bright green for visibility)
    let color = egui::Color32::from_rgb(255, 255, 0);

    for x in 0..width {
        img.pixels[x as usize + center_y * width as usize] = color;
    }

    for y in 0..height {
        img.pixels[center_x as usize + y * width as usize] = color;
    }

    // Draw circle outline
    // let radius = 100.;

    // Use Bresenham's circle algorithm
    let mut x = 0;
    let mut y = radius as i32;
    let mut d = 3 - 2 * (radius as i32);

    while y >= x {
        // Draw the eight octants
        draw_circle_points(img, center_x, center_y, x, y, color, width);

        if d > 0 {
            y -= 1;
            d += 4 * (x - y) + 10;
        } else {
            d += 4 * x + 6;
        }
        x += 1;
    }
}

// Helper function to draw the eight points of a circle at once
fn draw_circle_points(
    img: &mut egui::ColorImage,
    center_x: usize,
    center_y: usize,
    x: i32,
    y: i32,
    color: egui::Color32,
    width: usize,
) {
    let points = [
        (center_x + x as usize, center_y + y as usize),
        (center_x - x as usize, center_y + y as usize),
        (center_x + x as usize, center_y - y as usize),
        (center_x - x as usize, center_y - y as usize),
        (center_x + y as usize, center_y + x as usize),
        (center_x - y as usize, center_y + x as usize),
        (center_x + y as usize, center_y - x as usize),
        (center_x - y as usize, center_y - x as usize),
    ];

    for (px, py) in points {
        // Check bounds before drawing
        if px < img.width() && py < img.height() {
            img.pixels[px + py * width] = color;
        }
    }
}
