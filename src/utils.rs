use std::{error::Error, rc::Rc};

pub use css::setup_styling;
#[allow(unused_imports)]
use log::*;

use crate::{
    config::CONFIG,
    dbus::{IFace, IFaceRef, Reason},
    margins_update,
    types::RuntimeData,
};

mod css {
    use std::collections::HashMap;

    use gtk::{gdk, CssProvider};
    use log::{debug, info};

    pub fn setup_styling() {
        let settings = gtk::Settings::default().unwrap();
        settings.connect_gtk_theme_name_notify(|_| load_css());
        // FIXME this doesn't catch theme variant changing
        // settings.connect_gtk_application_prefer_dark_theme_notify(|_| load_css());
        load_css();
    }

    pub fn load_css() {
        let provider = CssProvider::new();

        let text = css_glob_export_string();
        let theme_colors = css_glob_export_colors(&text);

        let borders = theme_colors.get("borders").unwrap_or(&"gray");
        let theme_base_color = theme_colors.get("theme_base_color").unwrap_or(&"gray");

        info!(
            "Loading CSS with border color: {} and base color: {}",
            borders, theme_base_color
        );

        provider.load_from_data(&format!(
            "
    @keyframes slide {{
        from {{
            opacity: 0;
            transform: translateY(-20pt) scale(0.95);
        }}
        to {{
            opacity: 1;
            transform: translateY(0pt) scale(1);
        }}
    }}

    
    @keyframes slide_r {{
        from {{
            opacity: 1;
            transform: translateY(0pt) scale(1);
        }}

        to {{
            opacity: 0;
            transform: translateY(20pt) scale(0.95);
        }}
    }}
    

    /* Notification hacks. Watch out, here be dragons (it's me, I'm the dragon)
       In order to get smooth animations, I had to basically treat Adw::Window as a popover, meaning I had to manually style
       dialog-host and rip out the styles for windows. Libadwaita has a bit of hard coding in its CSS, so I had to copy that over.
       Good news! Still easy to theme as always, just change the color variables. Bad news! If libadwaita changes, I'm screwed.
       ~ spike */

    #notification {{
      box-shadow: unset;
      background: unset;
      padding: 8pt; /** TODO take from config edge padding */
      border-radius: 0px;
      outline: unset;
    }}

    #notification .content {{
        margin: 8pt;
        margin-top: 4pt;
        padding: 6pt 10pt;
    }}

    #notification > dialog-host {{
        background-color: var(--window-bg-color);
        /* copied from libadwaita */
        box-shadow: 0 0 8px 5px RGB(0 0 0 / 8%),
                0 0 5px 2px RGB(0 0 0 / 3%),
                0 0 0 1px RGB(0 0 0 / 2%);
        transition: background 0.3s ease;
        border: 1px solid var(--border-color);
        border-radius: 12pt;
    }}

    #notification.hide > dialog-host {{
      opacity: 0;
    }}

    #notification:not(.hide) > dialog-host {{
      opacity: 1;
      animation: slide 0.3s ease;
    }}

    #notification.closing > dialog-host {{
      animation: slide_r 0.3s ease;
    }}
    
    #notification.hover > dialog-host {{
      background-color: color-mix(in srgb, var(--window-fg-color) 5%, var(--window-bg-color));
    }}",
        ));

        gtk::style_context_add_provider_for_display(
            &gdk::Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        info!("CSS loaded and applied successfully.");
    }

    pub fn css_glob_export_string() -> String {
        let settings = gtk::Settings::default().unwrap();
        let theme_name = settings.gtk_theme_name().unwrap();
        let pref_dark = settings.is_gtk_application_prefer_dark_theme();

        debug!(
            "Loading theme: {}, Dark mode preference: {}",
            theme_name, pref_dark
        );

        let css_provider = gtk::CssProvider::new();
        css_provider.load_named(&theme_name, if pref_dark { Some("dark") } else { None });

        css_provider.to_string()
    }

    pub fn css_glob_export_colors(text: &str) -> HashMap<&str, &str> {
        text.lines()
            .map(|line| line.trim().trim_end_matches(";"))
            .filter(|line| line.starts_with("@define-color"))
            .filter_map(|line| {
                line.trim_start_matches("@define-color")
                    .trim()
                    .split_once(" ")
            })
            .collect()
    }
}

pub async fn close_hook(
    id: u32,
    died_from: Reason,
    iface: Rc<IFaceRef>,
    runtime_data: RuntimeData,
) {
    info!(
        "Close hook called for notification with ID: {}, Reason: {:?}",
        id, died_from
    );

    match IFace::notification_closed(iface.signal_context(), id, died_from).await {
        Ok(_) => info!("Notification closed successfully for ID: {}", id),
        Err(e) => error!(
            "Error while closing notification for ID: {}, Error: {:?}",
            id, e
        ),
    }

    runtime_data.borrow_mut().windows.remove(&id);
    margins_update(runtime_data.clone());

    debug!("Margins updated after closing notification with ID: {}", id);
}

pub fn logger_init() -> Result<(), Box<dyn Error>> {
    use sys_logger::{connected_to_journal, JournalLog};

    if connected_to_journal() {
        JournalLog::new()?.install()?;
    } else {
        env_logger::init();
    }
    log::set_max_level(CONFIG.lock().unwrap().log_level.into());
    Ok(())
}
