use std::{collections::HashMap, path::PathBuf};

use cosmic_text::{
    Attrs, Buffer, Color as TextColor, FamilyOwned, FontSystem, Metrics, Shaping, SwashCache, Weight, fontdb,
};

use super::pixels::{BYTES_PER_PIXEL, Color, blend_over, premultiplied_argb};

pub const ELLIPSIS: char = '…';
pub const MAXIMUM_CHARACTERS: usize = 2048;
const LINE_HEIGHT_RATIO: f32 = 1.25;
const SANS_SERIF_CANDIDATES: [&str; 6] = ["Noto Sans", "DejaVu Sans", "Liberation Sans", "Open Sans", "Adwaita Sans", "Cantarell"];
const SERIF_CANDIDATES: [&str; 4] = ["Noto Serif", "DejaVu Serif", "Liberation Serif", "Times New Roman"];
const MONOSPACE_CANDIDATES: [&str; 5] = ["Noto Sans Mono", "DejaVu Sans Mono", "Liberation Mono", "Adwaita Mono", "Courier New"];

#[derive(Clone, Debug, PartialEq)]
pub enum FontSource {
    Family(String),
    File(PathBuf),
}

#[derive(Clone, Debug, PartialEq)]
pub struct FontSpec {
    pub source: FontSource,
    pub size: f32,
    pub weight: u16,
}

impl Default for FontSpec {
    fn default() -> Self {
        Self { source: FontSource::Family("sans-serif".to_owned()), size: 14.0, weight: 400 }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
    pub shown: String,
}

pub struct TextBitmap {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u8>,
}

#[derive(Default)]
pub struct TextEngine {
    fonts: Option<FontSystem>,
    swash: Option<SwashCache>,
    family_of_file: HashMap<PathBuf, String>,
}

fn first_installed(database: &fontdb::Database, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|name| database.faces().any(|face| face.families.iter().any(|(family, _)| family == *name)))
        .map(|name| (*name).to_owned())
}

fn system_fonts() -> FontSystem {
    let mut database = fontdb::Database::new();
    database.load_system_fonts();
    if let Some(family) = first_installed(&database, &SANS_SERIF_CANDIDATES) {
        database.set_sans_serif_family(family);
    }
    if let Some(family) = first_installed(&database, &SERIF_CANDIDATES) {
        database.set_serif_family(family);
    }
    if let Some(family) = first_installed(&database, &MONOSPACE_CANDIDATES) {
        database.set_monospace_family(family);
    }
    FontSystem::new_with_locale_and_db("en-US".to_owned(), database)
}

fn single_line(text: &str) -> String {
    text.chars().take(MAXIMUM_CHARACTERS).map(|c| if c.is_control() { ' ' } else { c }).collect()
}

fn text_color(color: Color) -> TextColor {
    let byte = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    TextColor::rgb(byte(color[0]), byte(color[1]), byte(color[2]))
}

impl TextEngine {
    pub fn font_count(&mut self) -> usize {
        self.fonts().db().faces().count()
    }

    fn fonts(&mut self) -> &mut FontSystem {
        self.fonts.get_or_insert_with(system_fonts)
    }

    fn family(&mut self, source: &FontSource) -> Result<FamilyOwned, String> {
        match source {
            FontSource::Family(name) => Ok(match name.to_ascii_lowercase().as_str() {
                "sans-serif" => FamilyOwned::SansSerif,
                "serif" => FamilyOwned::Serif,
                "monospace" => FamilyOwned::Monospace,
                _ => FamilyOwned::Name(name.as_str().into()),
            }),
            FontSource::File(path) => {
                if let Some(name) = self.family_of_file.get(path) {
                    return Ok(FamilyOwned::Name(name.as_str().into()));
                }
                let database = self.fonts().db_mut();
                let ids = database.load_font_source(fontdb::Source::File(path.clone()));
                let name = ids
                    .first()
                    .and_then(|id| database.face(*id))
                    .and_then(|face| face.families.first())
                    .map(|(family, _)| family.clone())
                    .ok_or_else(|| format!("{} is not a font file", path.display()))?;
                self.family_of_file.insert(path.clone(), name.clone());
                Ok(FamilyOwned::Name(name.as_str().into()))
            }
        }
    }

    fn shape(&mut self, text: &str, font: &FontSpec, scale: f32) -> Result<(Buffer, TextMetrics), String> {
        let family = self.family(&font.source)?;
        let size = (font.size * scale).max(1.0);
        let fonts = self.fonts();
        let mut buffer = Buffer::new(fonts, Metrics::new(size, (size * LINE_HEIGHT_RATIO).round()));
        buffer.set_size(fonts, None, None);
        let attrs = Attrs::new().family(family.as_family()).weight(Weight(font.weight));
        buffer.set_text(fonts, text, &attrs, Shaping::Advanced, None);
        buffer.shape_until_scroll(fonts, false);
        let mut metrics = TextMetrics { width: 0.0, height: (size * LINE_HEIGHT_RATIO).round(), baseline: 0.0, shown: text.to_owned() };
        for run in buffer.layout_runs() {
            metrics.width = metrics.width.max(run.line_w);
            metrics.baseline = run.line_y;
        }
        Ok((buffer, metrics))
    }

    fn width(&mut self, text: &str, font: &FontSpec, scale: f32) -> Result<f32, String> {
        Ok(self.shape(text, font, scale)?.1.width)
    }

    fn ellipsized(&mut self, text: &str, font: &FontSpec, scale: f32, max_width: f32) -> Result<String, String> {
        if self.width(text, font, scale)? <= max_width {
            return Ok(text.to_owned());
        }
        let characters: Vec<char> = text.chars().collect();
        let with_ellipsis = |count: usize| characters[..count].iter().chain([&ELLIPSIS]).collect::<String>();
        let (mut fits, mut too_long) = (0, characters.len());
        while too_long - fits > 1 {
            let middle = (fits + too_long) / 2;
            if self.width(&with_ellipsis(middle), font, scale)? <= max_width {
                fits = middle;
            } else {
                too_long = middle;
            }
        }
        if fits == 0 && self.width(&ELLIPSIS.to_string(), font, scale)? > max_width {
            return Ok(String::new());
        }
        Ok(with_ellipsis(fits))
    }

    pub fn measure(&mut self, text: &str, font: &FontSpec, max_width: Option<f32>) -> Result<TextMetrics, String> {
        let text = single_line(text);
        let shown = match max_width {
            Some(max_width) => self.ellipsized(&text, font, 1.0, max_width)?,
            None => text,
        };
        let mut metrics = self.shape(&shown, font, 1.0)?.1;
        metrics.shown = shown;
        Ok(metrics)
    }

    pub fn extent(&mut self, shown: &str, font: &FontSpec, scale: f32) -> Result<(f32, f32), String> {
        let metrics = self.shape(shown, font, scale)?.1;
        Ok((metrics.width, metrics.height))
    }

    pub fn rasterize(
        &mut self,
        shown: &str,
        font: &FontSpec,
        color: Color,
        scale: f32,
        canvas: (u32, u32),
    ) -> Result<TextBitmap, String> {
        let (buffer, _) = self.shape(shown, font, scale)?;
        let mut pixels = vec![0; canvas.0 as usize * canvas.1 as usize * BYTES_PER_PIXEL];
        let fonts = self.fonts.as_mut().ok_or("the fonts are not loaded")?;
        let swash = self.swash.get_or_insert_with(SwashCache::new);
        buffer.draw(fonts, swash, text_color(color), |x, y, width, height, glyph| {
            let opacity = f32::from(glyph.a()) / 255.0 * color[3];
            let rgba = [glyph.r(), glyph.g(), glyph.b()].map(|channel| f32::from(channel) / 255.0);
            let source = premultiplied_argb([rgba[0], rgba[1], rgba[2], opacity], 1.0);
            for row in y.max(0)..(y + height as i32).min(canvas.1 as i32) {
                for column in x.max(0)..(x + width as i32).min(canvas.0 as i32) {
                    blend_over(&mut pixels, (row as usize * canvas.0 as usize + column as usize) * BYTES_PER_PIXEL, source);
                }
            }
        });
        Ok(TextBitmap { width: canvas.0, height: canvas.1, pixels })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WHITE: Color = [1.0, 1.0, 1.0, 1.0];

    fn engine() -> TextEngine {
        let mut engine = TextEngine::default();
        assert!(engine.font_count() > 0, "these tests need at least one installed font");
        engine
    }

    #[test]
    fn longer_text_is_wider_and_empty_text_has_no_width() {
        let mut engine = engine();
        let font = FontSpec::default();
        let short = engine.measure("ab", &font, None).unwrap();
        let long = engine.measure("abcdef", &font, None).unwrap();
        assert!(long.width > short.width && short.width > 0.0);
        assert_eq!(engine.measure("", &font, None).unwrap().width, 0.0);
    }

    #[test]
    fn a_bigger_font_makes_wider_and_taller_text() {
        let mut engine = engine();
        let small = FontSpec { size: 10.0, ..FontSpec::default() };
        let big = FontSpec { size: 20.0, ..FontSpec::default() };
        let (small, big) = (engine.measure("hello", &small, None).unwrap(), engine.measure("hello", &big, None).unwrap());
        assert!(big.width > small.width * 1.5);
        assert!(big.height > small.height * 1.5);
    }

    #[test]
    fn text_is_cut_with_an_ellipsis_to_fit() {
        let mut engine = engine();
        let font = FontSpec::default();
        let full = engine.measure("a very long window title indeed", &font, None).unwrap();
        let cut = engine.measure("a very long window title indeed", &font, Some(full.width / 2.0)).unwrap();
        assert!(cut.width <= full.width / 2.0);
        assert!(cut.shown.ends_with(ELLIPSIS) && cut.shown.len() > ELLIPSIS.len_utf8());
        let kept = cut.shown.trim_end_matches(ELLIPSIS);
        assert!("a very long window title indeed".starts_with(kept));
    }

    #[test]
    fn text_that_fits_is_left_alone() {
        let mut engine = engine();
        let font = FontSpec::default();
        let full = engine.measure("short", &font, None).unwrap();
        assert_eq!(engine.measure("short", &font, Some(full.width)).unwrap().shown, "short");
    }

    #[test]
    fn a_limit_narrower_than_the_ellipsis_shows_nothing() {
        let mut engine = engine();
        assert_eq!(engine.measure("something", &FontSpec::default(), Some(0.5)).unwrap().shown, "");
    }

    #[test]
    fn line_breaks_become_spaces() {
        let mut engine = engine();
        let metrics = engine.measure("a\nb\tc", &FontSpec::default(), None).unwrap();
        assert_eq!(metrics.shown, "a b c");
    }

    #[test]
    fn rasterized_text_has_ink_inside_the_canvas_and_none_outside() {
        let mut engine = engine();
        let font = FontSpec::default();
        let metrics = engine.measure("Hi", &font, None).unwrap();
        let canvas = (metrics.width.ceil() as u32 + 8, metrics.height.ceil() as u32);
        let bitmap = engine.rasterize("Hi", &font, WHITE, 1.0, canvas).unwrap();
        let inked: Vec<usize> = (0..bitmap.pixels.len() / 4).filter(|i| bitmap.pixels[i * 4 + 3] > 0).collect();
        assert!(inked.len() > 10);
        let rightmost = inked.iter().map(|i| i % canvas.0 as usize).max().unwrap();
        assert!(rightmost < metrics.width.ceil() as usize + 1);
    }

    #[test]
    fn colour_and_translucency_reach_the_pixels() {
        let mut engine = engine();
        let font = FontSpec { size: 30.0, ..FontSpec::default() };
        let bitmap = engine.rasterize("I", &font, [1.0, 0.0, 0.0, 0.5], 1.0, (40, 40)).unwrap();
        let strongest = bitmap.pixels.chunks(4).max_by_key(|p| p[3]).unwrap();
        assert!(strongest[3] > 100 && strongest[3] <= 128, "alpha {}", strongest[3]);
        assert_eq!(strongest[0], 0);
        assert_eq!(strongest[1], 0);
        assert!(strongest[2] > 100);
    }

    #[test]
    fn a_file_that_is_not_a_font_is_an_error() {
        let mut engine = engine();
        let font = FontSpec { source: FontSource::File("/nonexistent/font.ttf".into()), ..FontSpec::default() };
        assert!(engine.measure("x", &font, None).is_err());
    }
}
