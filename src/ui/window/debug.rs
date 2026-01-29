use crate::ui::{downcast, ViewContext, Window};
use eframe::egui;
use once_cell::sync::Lazy;
use std::cell::RefCell;
use std::rc::Rc;
use crate::app::DebugState;

pub struct DebugWindow {
    debug_state: Rc<RefCell<dyn DebugState>>,
    current_captcha: CaptchaTag,
}

impl DebugWindow {
    pub fn new(debug_state: Rc<RefCell<dyn DebugState>>) -> Self {
        DebugWindow {
            current_captcha: CaptchaTag::First,
            debug_state,
        }
    }
}

impl Window for DebugWindow {
    fn view(&mut self, ctx: &dyn ViewContext) {
        let ctx = downcast(ctx);

        egui::Window::new("Debug")
            .anchor(egui::Align2::LEFT_TOP, [0.0, 0.0])
            .show(ctx, |ui| {
                let old = self.current_captcha;
                for captcha in TEST_CAPTCHA.iter() {
                    ui.radio_value(&mut self.current_captcha, captcha.tag, captcha.answer);
                }
                if self.current_captcha != old {
                    let index = self.current_captcha as usize;
                    self.debug_state
                        .borrow_mut()
                        .set_captcha(TEST_CAPTCHA[index].base64);
                }
            });
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CaptchaTag {
    First = 0,
    Second,
}

#[derive(Debug)]
struct CaptchaItem {
    pub tag: CaptchaTag,
    pub answer: &'static str,
    pub base64: &'static str,
}

static TEST_CAPTCHA: Lazy<Vec<CaptchaItem>> = Lazy::new(|| {
    vec![
        CaptchaItem {
            tag: CaptchaTag::First,
            answer: "996547",
            base64: "iVBORw0KGgoAAAANSUhEUgAAAGQAAAAyCAMAAACd646MAAAAP1BMVEUAAAAAOnJjndUIQnpRi8OBu/N8tu4qZJwMRn6Lxf0KRHxSjMRalMwoYppmoNhrpd01b6cgWpI8dq4tZ58WUIhMzA4eAAAAAXRSTlMAQObYZgAAAapJREFUeJzsmM1yhCAMgJN1RmU86Pj+D9vpAiHEwAaxTA+bQ9WK+fKLZuEfysuyaO1kvAyUde2lWBZxBvbxTII4hDKA8YfiqncRnwiic2UKBomXbZp53TmAs8QAKgdPmxo4ooPOU6VEDwIEAKZpskNkB+mepDBhPJ8IDI/kSoXEI0tWZ2VoEIqdT1fC3QWR8QpDGHLfFyquC0QylMfsECRCyoU/R10hxa0hgpiUstAjQsp6vh4yRkuuqIgAgTWmVIOpQLg/NaX6Jd8DGAxSJMOdgq4SJSslBLkTgPCQ940/Fl+diELxWw5Rryz+6ZD5+OEjwK9w3KnjODQIUBEiwJwZ+v5X7SPg98H4GggWlzxJV/M8i2iw3C8FjAPuff642FL8X8lgUVkWlZLMELVISZAk0BhQ84SZq9JZ0V4oBqZOCvVJHY+8waFCsFLCyitW06D3+QMvA4u2MmZvJldMLmxm+/6ZcghFrWZZPIk7QUBUGFs7PlGM67ath2KVEYyvfOWG1Ie1hxiVYY2ke86wMIbM72Mow38l4N8ULWNoE4NB2obdJspPAAAA//9aeATJZ1KZSAAAAABJRU5ErkJggg==",
        },
        CaptchaItem {
            tag: CaptchaTag::Second,
            answer: "682485",
            base64: "iVBORw0KGgoAAAANSUhEUgAAAGQAAAAyCAMAAACd646MAAAAP1BMVEUAAAARfGBu2b0ahWkok3d+6c1NuJxBrJBl0LRPup4Qe19s17seiW0CbVFTvqIynYEch2t+6c0qlXk0n4NFsJTJ6I4rAAAAAXRSTlMAQObYZgAAAbtJREFUeJzsl93usyAMxtstmQkjWYj3f69vhiJtaeVDZt6Df0/GJuuvD31Ahbvicwfjc4HybKacXHtXGE+d4lvZkfGuUXSGr1Bwi0TpKYlQqgyglPnBU0fafAYgI+IPJIl83/xutphvM4CU/mW4nYIAU0ShaHn8dBt7v3ANgqTHVEm6lqqIn10bTeoQAwJhP5gb7dHAkEJALNEx8np/Ho8aJTYXRd081TF2wh2tSsjyI5WmCXHOYf5Djx+Q9/aotewUoOPT81SNyBY/nVqibCkn5S0qQ4UhiHkCE+DlUSmz00qMxcmOR9HK7Zv3XjKQVngGyV40HH84zUvBUCS3laSw9i7qxpEK6Bw500LybGUj0w/UgXyMuzfqhCzGBB1DQL5hiEHbKOUOyZy8O8KJZ9V465iSnhsdQlAadMYoHn+0v4hGh/2eVhzZjUpU/aXBYv8Wu6qWMJaAH3LLslyCgNFQZq6rSkjOvE8Q2YFoOnEcl89vcb788rETuu9eWjS8g1xjPO13kGkRAc2MMErpmBuCTXGDfIViM9w8ih13MP7iP4xXx9x1lPFqpqzrOky5gdERNyDG418AAAD//3/dBjfl+kg/AAAAAElFTkSuQmCC",
        },
    ]
});
