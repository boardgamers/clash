#[cfg(target_arch = "wasm32")]
mod browser {
    use macroquad::prelude::*;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = window, js_name = bgsTranslate)]
        pub fn translate(text: &str) -> String;
        #[wasm_bindgen(js_namespace = window, js_name = bgsWrapText)]
        pub fn wrap(text: &str, width: f32, size: u16) -> String;
        #[wasm_bindgen(js_namespace = window, js_name = bgsTextEnabled)]
        pub fn enabled() -> bool;
        #[wasm_bindgen(js_namespace = window, js_name = bgsTextRevision)]
        fn revision() -> u32;
        #[wasm_bindgen(js_namespace = window, js_name = bgsTextMetrics)]
        fn metrics(text: &str, size: u16) -> Vec<f32>;
        #[wasm_bindgen(js_namespace = window, js_name = bgsTextPixels)]
        fn pixels(text: &str, size: u16) -> Vec<u8>;
    }

    struct TextImage {
        texture: Texture2D,
        metrics: Vec<f32>,
    }
    #[derive(Default)]
    struct Cache {
        revision: u32,
        entries: HashMap<(String, u16), TextImage>,
    }
    thread_local! { static CACHE: RefCell<Cache> = RefCell::new(Cache::default()); }

    pub fn measure(text: &str, size: u16) -> TextDimensions {
        let m = metrics(text, size);
        TextDimensions {
            width: m[4],
            height: m[5],
            offset_y: m[6],
        }
    }

    pub fn draw(text: &str, size: u16, x: f32, y: f32, color: Color) {
        CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let current = revision();
            if cache.revision != current || cache.entries.len() > 1000 {
                cache.entries.clear();
                cache.revision = current;
            }
            let entry = cache
                .entries
                .entry((text.to_owned(), size))
                .or_insert_with(|| {
                    let m = metrics(text, size);
                    let texture = Texture2D::from_rgba8(
                        (m[0] * 2.0) as u16,
                        (m[1] * 2.0) as u16,
                        &pixels(text, size),
                    );
                    texture.set_filter(FilterMode::Linear);
                    TextImage {
                        texture,
                        metrics: m,
                    }
                });
            let m = &entry.metrics;
            draw_texture_ex(
                &entry.texture,
                x - m[2],
                y - m[3],
                color,
                DrawTextureParams {
                    dest_size: Some(vec2(m[0], m[1])),
                    ..Default::default()
                },
            );
        });
    }
}

pub(crate) fn translate(text: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        browser::translate(text)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        text.to_owned()
    }
}

pub(crate) fn measure(text: &str, size: u16) -> Option<macroquad::prelude::TextDimensions> {
    #[cfg(target_arch = "wasm32")]
    if browser::enabled() {
        return Some(browser::measure(text, size));
    }
    let _ = (text, size);
    None
}

pub(crate) fn draw(
    text: &str,
    size: u16,
    x: f32,
    y: f32,
    color: macroquad::prelude::Color,
) -> bool {
    #[cfg(target_arch = "wasm32")]
    if browser::enabled() {
        browser::draw(text, size, x, y, color);
        return true;
    }
    let _ = (text, size, x, y, color);
    false
}

pub(crate) fn wrap(text: &str, width: f32, size: u16) -> Option<Vec<String>> {
    #[cfg(target_arch = "wasm32")]
    if browser::enabled() {
        return serde_json::from_str(&browser::wrap(text, width, size)).ok();
    }
    let _ = (text, width, size);
    None
}
