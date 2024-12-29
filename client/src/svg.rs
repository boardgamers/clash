/*
    rasterize svg to png image
*/
use macroquad::prelude::{ImageFormat, Texture2D};
use resvg::tiny_skia::Pixmap;
use resvg::usvg::{Options, Transform};

pub fn to_png(svg_str: &str) -> Vec<u8> {
    let opt = Options {
        // dpi: 6.0,
        // shape_rendering: resvg::usvg::ShapeRendering::OptimizeSpeed,
        ..Default::default()
    };
    let tree = resvg::usvg::Tree::from_str(svg_str, &opt).unwrap();
    // let mut fontdb = fontdb::Database::new();
    // fontdb.load_system_fonts();
    // tree.convert_text(&fontdb, opt.keep_named_groups);
    let pixmap_size = tree.size(); //.scale_to(Size::from_wh(50.0, 50.0).unwrap());
    let mut pixmap = Pixmap::new(pixmap_size.width() as u32, pixmap_size.height() as u32).unwrap();

    resvg::render(
        &tree,
        // resvg::usvg::FitTo::Original,
        Transform::default(),
        // resvg::tiny_skia::Transform::from_scale(0.1, 0.1),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().unwrap()
}

/*
    rasterize svg and create Texture2D
*/
pub fn to_texture(svg_str: &str) -> Texture2D {
    let png_data = to_png(svg_str);
    Texture2D::from_file_with_format(&png_data, Some(ImageFormat::Png))
}

pub fn icon(vec: &str) -> Texture2D {
    to_texture(vec)
}
