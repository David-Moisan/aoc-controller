use egui::{CentralPanel, ComboBox, RichText, Slider, TopBottomPanel, Ui};
use ddc_hi::Display;
use crate::monitor::{
    self,
    ColorChannels, ColorPreset, GameSettings, LuminanceSettings,
    MonitorInfo, Overdrive,
};

#[derive(PartialEq)]
enum Tab {
    Luminance,
    Game,
    Colour,
}

pub struct MonitorApp {
    // --- Monitor management ---
    monitors:        Vec<MonitorInfo>,   // list detected at startup
    selected_index:  usize,             // which monitor is active
    display:         Option<Display>,   // the open DDC connection

    // --- Current settings (what we last read from the monitor) ---
    luminance:       LuminanceSettings,
    game:            GameSettings,
    colour:          ColorChannels,

    // --- UI state ---
    active_tab:      Tab,
    status_message:  String,  // shown at the bottom — "Brightness set to 80" etc.
    loading:         bool,    // true while we're reading from the monitor
}

impl MonitorApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Slightly larger default font — easier to read for a control panel
        let mut style = (*cc.egui_ctx.style()).clone();
        style.text_styles.iter_mut().for_each(|(_, font)| {
            font.size = font.size.max(14.0);
        });
        cc.egui_ctx.set_style(style);

        let monitors = monitor::enumerate_monitors();

        // Try to open the first monitor and read its settings right away
        let (display, luminance, game, colour, status) =
            Self::load_monitor(0, &monitors);

        Self {
            monitors,
            selected_index: 0,
            display,
            luminance,
            game,
            colour,
            active_tab: Tab::Luminance,
            status_message: status,
            loading: false,
        }
    }

    fn load_monitor(
        index: usize,
        monitors: &[MonitorInfo],
    ) -> (Option<Display>, LuminanceSettings, GameSettings, ColorChannels, String) {
        // Safe defaults — shown if reading fails
        let default_luminance = LuminanceSettings {
            brightness: 50, contrast: 50,
            color_preset: ColorPreset::Preset5,
            dcr: false, hdr_mode: false,
        };
        let default_game = GameSettings {
            overdrive: Overdrive::Off,
            game_color: 50,
        };
        let default_colour = ColorChannels { red: 50, green: 50, blue: 50 };

        if monitors.is_empty() {
            return (None, default_luminance, default_game, default_colour,
                "No monitors detected.".to_string());
        }

        match monitor::open_monitor(index) {
            Err(e) => (None, default_luminance, default_game, default_colour,
                format!("Failed to open monitor: {}", e)),

            Ok(mut d) => {
                // Read all three setting groups — if any fail, use defaults
                let luminance = monitor::read_luminance(&mut d)
                    .unwrap_or(default_luminance);
                let game = monitor::read_game(&mut d)
                    .unwrap_or(default_game);
                let colour = monitor::read_color_channels(&mut d)
                    .unwrap_or(default_colour);

                (Some(d), luminance, game, colour,
                    format!("Loaded: {} ({})",
                        monitors[index].model,
                        monitors[index].serial))
            }
        }
    }

    fn switch_monitor(&mut self, new_index: usize) {
        self.loading = true;
        // Drop the old Display — this closes the I²C connection cleanly.
        // In Rust, dropping = the value goes out of scope = destructor runs.
        self.display = None;

        let (display, luminance, game, colour, status) =
            Self::load_monitor(new_index, &self.monitors);

        self.selected_index = new_index;
        self.display = display;
        self.luminance = luminance;
        self.game = game;
        self.colour = colour;
        self.status_message = status;
        self.loading = false;
    }

    fn apply<F>(&mut self, label: &str, f: F)
    where
        // F is any function that takes a &mut Display and returns Result<()>
        F: FnOnce(&mut Display) -> anyhow::Result<()>,
    {
        match &mut self.display {
            None => self.status_message = "No monitor connected.".to_string(),
            Some(d) => match f(d) {
                Ok(_)  => self.status_message = format!("{} applied.", label),
                Err(e) => self.status_message = format!("Error: {}", e),
            }
        }
    }
}

impl eframe::App for MonitorApp {
    /// Called every frame. We build the entire UI here declaratively.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- Top bar: monitor selector ---
        TopBottomPanel::top("monitor_selector").show(ctx, |ui| {
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Monitor").strong());

                // ComboBox is a dropdown. We show the serial so you can tell
                // your two identical AOC screens apart.
                let current_label = if self.monitors.is_empty() {
                    "No monitors found".to_string()
                } else {
                    format!("{} — {}",
                        self.monitors[self.selected_index].model,
                        self.monitors[self.selected_index].serial)
                };

                let previous_index = self.selected_index;

                ComboBox::from_id_source("monitor_combo")
                    .selected_text(&current_label)
                    .width(320.0)
                    .show_ui(ui, |ui| {
                        let monitors = self.monitors.clone();
                        for m in &monitors {
                            let label = format!("{} — {}", m.model, m.serial);
                            // selectable_value writes m.index into self.selected_index
                            // when the user clicks — we detect the change AFTER the combo closes
                            ui.selectable_value(
                                &mut self.selected_index,
                                m.index,
                                label,
                            );
                        }
                    });
                
                if self.selected_index != previous_index {
                    self.switch_monitor(self.selected_index);
                }

                // Refresh button — re-reads current settings from the monitor
                if ui.button("Refresh").clicked() {
                    self.switch_monitor(self.selected_index);
                }
            });
            ui.add_space(4.0);
        });

        // --- Bottom bar: status message ---
        TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.label(
                RichText::new(&self.status_message)
                    .color(if self.status_message.starts_with("Error") {
                        egui::Color32::RED
                    } else {
                        ui.visuals().text_color()
                    })
            );
            ui.add_space(4.0);
        });

        // --- Central panel: tab bar + active tab content ---
        CentralPanel::default().show(ctx, |ui| {
            if self.loading {
                ui.centered_and_justified(|ui| {
                    ui.label("Loading monitor settings...");
                });
                return;
            }

            // Tab selector
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.active_tab, Tab::Luminance, "Luminance");
                ui.selectable_value(&mut self.active_tab, Tab::Game,      "Game");
                ui.selectable_value(&mut self.active_tab, Tab::Colour,    "Colour");
            });

            ui.separator();
            ui.add_space(8.0);

            // Render the active tab — we pass `ui` down to helper methods
            match self.active_tab {
                Tab::Luminance => self.show_luminance_tab(ui),
                Tab::Game      => self.show_game_tab(ui),
                Tab::Colour    => self.show_colour_tab(ui),
            }
        });
    }
}

impl MonitorApp {
    fn show_luminance_tab(&mut self, ui: &mut Ui) {
        ui.heading("Luminance");
        ui.add_space(12.0);

        // --- Brightness slider ---
        // We store the old value before the slider so we can detect a change.
        // `egui::Slider` modifies the value in-place via a mutable reference.
        let old_brightness = self.luminance.brightness;
        ui.label("Brightness");
        ui.add(
            Slider::new(&mut self.luminance.brightness, 0..=100)
                .suffix("%")
                .show_value(true)
        );
        // Only send the DDC command when the value actually changed —
        // not every frame, which would spam the I²C bus at 60Hz
        if self.luminance.brightness != old_brightness {
            let v = self.luminance.brightness;
            self.apply("Brightness", |d| monitor::set_brightness(d, v));
        }

        ui.add_space(8.0);

        // --- Contrast slider ---
        let old_contrast = self.luminance.contrast;
        ui.label("Contrast");
        ui.add(
            Slider::new(&mut self.luminance.contrast, 0..=100)
                .suffix("%")
                .show_value(true)
        );
        if self.luminance.contrast != old_contrast {
            let v = self.luminance.contrast;
            self.apply("Contrast", |d| monitor::set_contrast(d, v));
        }

        ui.add_space(16.0);

        // --- Color preset dropdown ---
        ui.label("Color preset");
        let old_preset = self.luminance.color_preset;
        // We clone the current label to avoid borrow issues inside the closure
        ComboBox::from_label("")
            .selected_text(self.luminance.color_preset.label())
            .show_ui(ui, |ui| {
                for preset in ColorPreset::all() {
                    ui.selectable_value(
                        &mut self.luminance.color_preset,
                        *preset,
                        preset.label(),
                    );
                }
            });
        if self.luminance.color_preset != old_preset {
            let p = self.luminance.color_preset;
            self.apply("Color preset", |d| monitor::set_color_preset(d, p));
        }

        ui.add_space(16.0);

        // --- DCR toggle ---
        // `checkbox` takes a &mut bool and a label — that's it
        let old_dcr = self.luminance.dcr;
        ui.checkbox(&mut self.luminance.dcr, "DCR (Dynamic Contrast Ratio)");
        if self.luminance.dcr != old_dcr {
            let v = self.luminance.dcr;
            self.apply("DCR", |d| monitor::set_dcr(d, v));
        }

        ui.add_space(4.0);

        // --- HDR toggle ---
        let old_hdr = self.luminance.hdr_mode;
        ui.checkbox(&mut self.luminance.hdr_mode, "HDR Mode");
        if self.luminance.hdr_mode != old_hdr {
            let v = self.luminance.hdr_mode;
            self.apply("HDR mode", |d| monitor::set_hdr_mode(d, v));
        }
    }

    fn show_game_tab(&mut self, ui: &mut Ui) {
        ui.heading("Game");
        ui.add_space(12.0);

        // --- Overdrive dropdown ---
        ui.label("Overdrive (response time)");
        let old_od = self.game.overdrive;
        ComboBox::from_label(" ")
            .selected_text(self.game.overdrive.label())
            .show_ui(ui, |ui| {
                for od in Overdrive::all() {
                    ui.selectable_value(
                        &mut self.game.overdrive,
                        *od,
                        od.label(),
                    );
                }
            });
        if self.game.overdrive != old_od {
            let o = self.game.overdrive;
            self.apply("Overdrive", |d| monitor::set_overdrive(d, o));
        }

        ui.add_space(16.0);

        // --- Game color slider ---
        let old_gc = self.game.game_color;
        ui.label("Game color (saturation boost)");
        ui.add(
            Slider::new(&mut self.game.game_color, 0..=100)
                .show_value(true)
        );
        if self.game.game_color != old_gc {
            let v = self.game.game_color;
            self.apply("Game color", |d| monitor::set_game_color(d, v));
        }
    }

    fn show_colour_tab(&mut self, ui: &mut Ui) {
        ui.heading("Colour channels");
        ui.add_space(12.0);

        // Helper macro to avoid repeating the same 8-line pattern 3 times.
        // Macros in Rust start with `macro_rules!` and are expanded at compile
        // time — like a code template. The $x:expr captures any expression.
        macro_rules! colour_slider {
            ($label:expr, $field:expr, $apply_fn:expr) => {
                let old = $field;
                ui.label($label);
                ui.add(Slider::new(&mut $field, 0..=100).suffix("%").show_value(true));
                if $field != old {
                    let v = $field;
                    self.apply($label, |d| $apply_fn(d, v));
                }
                ui.add_space(8.0);
            };
        }

        colour_slider!("Red",   self.colour.red,   monitor::set_red);
        colour_slider!("Green", self.colour.green,  monitor::set_green);
        colour_slider!("Blue",  self.colour.blue,   monitor::set_blue);
    }
}