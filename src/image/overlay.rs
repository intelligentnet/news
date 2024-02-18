use image::DynamicImage;
use image::Rgba;
use imageproc::distance_transform::Norm;
use imageproc::drawing::draw_text_mut;
use imageproc::morphology::dilate_mut;
use rusttype::{Font, Scale};

pub fn overlay_text(text: &str, img_data: &[u8]) {
    let sb_img = image::load_from_memory(img_data).unwrap();

    let mut image = sb_img.to_rgba8();

    let mut image2: DynamicImage = DynamicImage::new_luma8(image.width(), image.height());

    //let font: &[u8] = include_bytes!(".fonts/DejaVuSans.ttf") as &[u8];
    let font: &[u8] = include_bytes!("/d/ide/crash/news/DejaVuSans.ttf") as &[u8];
    let font = Font::try_from_bytes(font).unwrap();

    let scale = Scale {
        x: image.width() as f32 * 0.2,
        y: image.height() as f32 * 0.2,
    };

    let x = (image.width() as f32 * 0.10) as i32;
    let y = (image.width() as f32 * 0.10) as i32;
    draw_text_mut(
        &mut image2,
        Rgba([255u8, 255u8, 255u8, 255u8]),
        x,
        y,
        scale,
        &font,
        text,
    );

    let mut image2 = image2.to_luma8();
    dilate_mut(&mut image2, Norm::LInf, 4u8);

    for x in 0..image2.width() {
        for y in 0..image2.height() {
            let pixval = 255 - image2.get_pixel(x, y).0[0];
            if pixval != 255 {
                let new_pix = Rgba([pixval, pixval, pixval, 255]);
                image.put_pixel(x, y, new_pix);
            }
        }
    }

    draw_text_mut(
        &mut image,
        Rgba([255u8, 255u8, 255u8, 255u8]),
        x,
        y,
        scale,
        &font,
        text,
    );
}
