use crate::log_ui;
use crate::render_context::RenderContext;

pub(crate) struct MultilineText {
    pub width: f32,
    pub text: Vec<String>,
}

impl MultilineText {
    pub(crate) fn default() -> Self {
        MultilineText {
            width: 500.,
            text: vec![],
        }
    }

    pub(crate) fn of(rc: &RenderContext, text: &str) -> Self {
        let mut t = Self::default();
        t.add(rc, text);
        t
    }

    pub(crate) fn from(rc: &RenderContext, text: &[String]) -> Self {
        let mut t = Self::default();
        for label in text {
            t.add(rc, label);
        }
        t
    }

    pub(crate) fn add(&mut self, rc: &RenderContext, label: &str) {
        log_ui::multiline_label(rc.state, label, self.width, |_, line: &str| {
            self.text.push(line.to_string());
        });
    }
}
