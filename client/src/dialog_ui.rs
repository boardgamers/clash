use macroquad::hash;
use macroquad::math::vec2;
use macroquad::ui::{root_ui, Ui};

pub fn active_dialog_window<F: FnOnce(&mut Ui)>(f: F) {
    root_ui().window(hash!(), vec2(20., 510.), vec2(400., 80.), f);
}
