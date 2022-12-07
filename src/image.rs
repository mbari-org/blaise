use image::{DynamicImage, GenericImageView, ImageResult};
use std::path::Path;

use log::debug;

pub fn load_image<Q: AsRef<Path>>(path: Q) -> ImageResult<DynamicImage> {
    debug!("loading image from {:?}", path.as_ref());
    image::open(path)
}

pub fn crop_image(img: &mut DynamicImage, x: u32, y: u32, width: u32, height: u32) -> DynamicImage {
    debug!("cropping image ...");
    img.crop(x, y, width, height)
}

pub fn resize_image(img: &DynamicImage, width: u32, height: u32) -> Option<DynamicImage> {
    // given errors noted here, and that `resize_exact` does not return a Result,
    // just checking for the image to not be empty:
    if img.width() > 0u32 && img.height() > 0u32 {
        debug!("resizing image ...");
        let filter = image::imageops::FilterType::Lanczos3;
        Some(img.resize_exact(width, height, filter))
    } else {
        None
    }
}

pub fn save_image<Q: AsRef<Path>>(img: DynamicImage, out_path: Q) {
    if let Err(e) = img.save(&out_path) {
        eprintln!("error saving {:?}: {:?}", out_path.as_ref(), e);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;

    static OUT_DIR: &str = "data/out";
    static INIT: Once = Once::new();
    fn init() {
        INIT.call_once(|| {
            std::fs::create_dir_all(OUT_DIR).unwrap();
        });
    }

    fn get_image() -> DynamicImage {
        load_image("data/imgs/IMG_TEST.png").unwrap()
    }

    #[test]
    fn crop() {
        init();

        let xmin = 2448;
        let ymin = 1389;
        let xmax = 2648;
        let ymax = 1589;
        let x = xmin;
        let y = ymin;
        let width = xmax - xmin;
        let height = ymax - ymin;
        let mut img = get_image();
        crop_image(&mut img, x, y, width, height);
    }
}
