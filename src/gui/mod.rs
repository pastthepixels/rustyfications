pub mod utils;
pub mod window;

use gtk::prelude::*;

pub fn build_ui(application: &adw::Application) {
    let w = adw::Window::new();
    w.set_application(Some(application));
}
