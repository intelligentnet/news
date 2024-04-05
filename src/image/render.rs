//use std::path::Path;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::fs::File;
use ril::prelude::*;
use crate::llm::gpt::{truncate, truncate_sentence};
use image::{imageops::overlay, GenericImageView, Rgba};
use imageproc::drawing::draw_text_mut;

pub const PAGE_TOTAL: usize = 5;

fn image_entry(image: &mut Image<ril::Rgb>, font : &Font, colour: ril::Rgb, w: usize, h: usize, text: &str, centre: bool) {
    let layout = TextLayout::new();
    // Shorthand for centering horizontally and vertically
    let layout = if centre { layout.centered() } else { layout };
    let layout = layout.with_wrap(WrapStyle::Word) // RIL supports word wrapping
        // This is the width to wrap text at.
        .with_width(image.width() - 220)
        // Position the anchor at the center of the image if required
        .with_position(if centre { image.width() / 2 } else { 25 + w as u32}, h as u32)
        .with_segment(&TextSegment::new(font, text, colour));

    image.draw(&layout);
}

//const WIDTH: u32 = 1920;
const HEIGHT: usize = 1080;
const OFFSET: usize = 35;

pub fn mk_filename(file: &str) -> String {
    format!("gen/{}.png", truncate(file, 100).replace(' ', "_").to_lowercase())
}

/*
pub fn mk_image(prompt: &str, title_text: &[(String, String, String)], len: usize, centre: bool) -> Result<String, String> {
    let ttl = title_text.len();
    if len == 0 || ttl == 0 {
        return Err(format!("No news available for query: {prompt}"));
    }

    let bold_fn = "Roboto-Bold.ttf";
    let font_fn = "Roboto-Regular.ttf";
    let head_sz = 34.0;
    let font_sz = 22.0;
    let big_bold = Font::open(bold_fn, head_sz).map_err(|err| format!("{}: {}", bold_fn, err))?;
    let bold = Font::open(bold_fn, font_sz).map_err(|err| format!("{}: {}", bold_fn, err))?;
    let font = Font::open(font_fn, font_sz).map_err(|err| format!("{}: {}", font_fn, err))?;

        //for i in (0 .. ttl).step_by(len as usize) {
    let out = {
            //let mut i = 0;
            let i = 0;
            let l = len.min(ttl - i);
            let hh = (HEIGHT - OFFSET) / l;
            let sub = hh / 12;

            let red = Rgb::new(255, 0, 0);
            //let magenta = Rgb::new(255, 0, 255);
            let grey = Rgb::new(150, 150, 150);

            let back_fn = "big_wood.png";
            let mut image = Image::<Rgb>::open(back_fn).map_err(|err| format!("{}: {}", back_fn, err))?;

            image_entry(&mut image, &big_bold, grey, 0, 0, &format!("News for: {prompt}"), centre);

            for item in 0 .. l {
                let hp = item * hh + OFFSET;
                let t1 = hp + sub;
                let t2 = hp + sub * 4;

                let it = &title_text[i + item];
                image_entry(&mut image, &bold, red, 0, t1, truncate(&it.0, 16), centre);

                image_entry(&mut image, &bold, Rgb::white(), 200, t1, &it.1, centre);
                image_entry(&mut image, &font, Rgb::white(), 0, t2, truncate_sentence(&it.2, 700), centre);
            }

            let out = mk_filename(prompt);

            image.save_inferred(&out).map_err(|err| err.to_string())?;

            if out.contains(".png") {
                out.to_string()
            } else {
                "NOT FOUND".into()
            }
        };

    Ok(out)
}
*/

pub fn mk_image_with_thumbnails(prompt: &str, title_text: &[(String, String, String)], len: usize, centre: bool) -> Result<String, String> {
    let ttl = title_text.len();
    if len == 0 || ttl == 0 {
        return Err(format!("No news available for query: {prompt}"));
    }

    let bold_fn = "Roboto-Bold.ttf";
    let font_fn = "Roboto-Regular.ttf";
    let head_sz = 34.0;
    let font_sz = 22.0;
    let big_bold = Font::open(bold_fn, head_sz).map_err(|err| format!("{}: {}", bold_fn, err))?;
    let bold = Font::open(bold_fn, font_sz).map_err(|err| format!("{}: {}", bold_fn, err))?;
    let font = Font::open(font_fn, font_sz).map_err(|err| format!("{}: {}", font_fn, err))?;

        //for i in (0 .. ttl).step_by(len as usize) {
    let out = {
            //let mut i = 0;
            let i = 0;
            let l = len.min(ttl - i);
            let hh = (HEIGHT - OFFSET) / l;
            let sub = hh / 12;

            let grey = Rgb::new(150, 150, 150);

            let back_fn = "big_wood.png";
            let mut image = Image::<Rgb>::open(back_fn).map_err(|err| format!("{}: {}", back_fn, err))?;

            image_entry(&mut image, &big_bold, grey, 0, 0, &format!("News for: {prompt}"), centre);

            for item in 0 .. l {
                let hp = item * hh + OFFSET;
                let t1 = hp + sub;
                let t2 = hp + sub * 4;

                let it = &title_text[i + item];
                //image_entry(&mut image, &bold, red, 0, t1, truncate(&it.0, 16), centre);
                let tn = thumbnail(200, &it.0);
//println!("{} {}", it.0, &tn);
                let thumb = Image::<Rgb>::open(tn.clone()).map_err(|e| format!("{}: {}", tn, e))?;
                image.paste(5, t1 as u32, &thumb);
                image_entry(&mut image, &bold, Rgb::white(), 200, t1, &it.1, centre);
                image_entry(&mut image, &font, Rgb::white(), 200, t2 - 15, truncate_sentence(&it.2, 700), centre);
            }

            let out = mk_filename(prompt);

            image.save_inferred(&out).map_err(|err| err.to_string())?;

            if out.contains(".png") {
                out.to_string()
            } else {
                "NOT FOUND".into()
            }
        };

    Ok(out)
}

pub fn use_image(prompt: &str, title_text: &str) -> Result<String, String> {
    let mut img = image::open("big_wood.png").unwrap();

    // Load a font.
    let font = Vec::from(include_bytes!("../../Roboto-Regular.ttf") as &[u8]);
    let font = rusttype::Font::try_from_vec(font).unwrap();

    let font_size = 40.0;
    let scale = rusttype::Scale {
        x: font_size,
        y: font_size,
    };

    // draw text
    if prompt != title_text {
        draw_text_mut(
            &mut img,
            Rgba([255u8, 0u8, 0u8, 255u8]),
            5, // x
            0, // y
            scale,
            &font,
            prompt, // Must be of type &str
        );
    }
    draw_text_mut(
        &mut img,
        Rgba([255u8, 255u8, 255u8, 255u8]),
        if prompt != title_text { 300 } else { 10 }, // x
        0, // y
        scale,
        &font,
        truncate_sentence(title_text,
            if prompt != title_text { 770 } else { 1060 }) // Must be of type &str
    );

    // overlay another image on top of the image
    let file = &mk_filename(prompt);
    let tile = image::open(file).unwrap();
    let (w, h) = img.dimensions();
    let (w2, h2) = tile.dimensions();
    overlay(&mut img, &tile, (w / 2 - w2 / 2).into(), ((h / 2 - h2 / 2) / 4 * 7).into());

    // re-output a new image
    img.save(file).unwrap();

    // make sure correct mount point is used
    Ok(file.replace("gen/", "pic/"))
}

fn thumbnail(size: u32, url: &str) -> String {
    fn hashname(url: &str) -> i64 {
        let mut hasher = DefaultHasher::new();

        url.hash(&mut hasher);

        (hasher.finish() >> 1) as i64
    }

    fn get_remote(size: u32, url: &str) -> Result<String, String> {
        let file_no = hashname(url);
        let file_name = format!("gen/{file_no}.png");

        if ! std::path::Path::new(&file_name).exists() {
            let img_bytes = reqwest::blocking::get(url)
                .map_err(|e| format!("{}: {}", url, e))?
                .bytes()
                .map_err(|e| format!("{}: {}", url, e))?;

            let img = image::load_from_memory(&img_bytes)
                .map_err(|e| format!("{}: {}", url, e))?;

            let scaled = img.thumbnail(size, size);
            let mut output = File::create(file_name)
                .map_err(|e| format!("{}: {}", url, e))?;

            scaled.write_to(&mut output, image::ImageFormat::Png)
                .map_err(|e| format!("{}: {}", url, e))?;

        }

        Ok(url.to_string())
    }

    let file_no = if url == "no_thumbnail" {
           hashname(url)
       } else {
           match get_remote(size, url) {
               Ok(url) => hashname(&url),
               Err(e) => {
                   eprintln!("{e}");

                   hashname("no_thumbnail")
               }
           }
       };

    format!("gen/{file_no}.png")
}
