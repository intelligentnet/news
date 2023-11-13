use ril::prelude::*;
use crate::llm::gpt::{truncate, truncate_sentence};

pub const PAGE_TOTAL: u32 = 5;

fn image_entry(image: &mut Image<ril::Rgb>, font : &Font, colour: ril::Rgb, w: u32, h: u32, text: &str, centre: bool) {
    let layout = TextLayout::new();
    // Shorthand for centering horizontally and vertically
    let layout = if centre { layout.centered() } else { layout };
    let layout = layout.with_wrap(WrapStyle::Word) // RIL supports word wrapping
        // This is the width to wrap text at.
        .with_width(image.width() - 50)
        // Position the anchor at the center of the image if required
        .with_position(if centre { image.width() / 2 } else { 25 + w }, h)
        .with_segment(&TextSegment::new(font, text, colour));

    image.draw(&layout);
}

//const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;
const OFFSET: u32 = 35;

pub fn mk_filename(prompt: &str) -> String {
    format!("gen/{}.png", prompt.replace(' ', "_"))
}

pub fn mk_image(prompt: &str, title_text: &Vec<(String, String, String)>, len: u32, centre: bool) -> Result<String, String> {
    let ttl = title_text.len() as u32;
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

                let it = &title_text[(i + item) as usize];
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
