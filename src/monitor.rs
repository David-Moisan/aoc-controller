use anyhow::{Context, Result};
use ddc_hi::{Ddc, Display};

// ---------------------------------------------------------------------------
// VCP codes — fully resolved from your 27B2G5 capability scan
// ---------------------------------------------------------------------------

pub struct VcpCode;
impl VcpCode {
    // Luminance
    pub const BRIGHTNESS:    u8 = 0x10; // 0–100
    pub const CONTRAST:      u8 = 0x12; // 0–100
    pub const COLOR_PRESET:  u8 = 0x14; // 0–11, see EcoMode enum
    pub const DCR:           u8 = 0x1E; // 0=off, 1=on
    pub const HDR_MODE:      u8 = 0x1F; // 0=off, 1=on

    // Game / image
    pub const OVERDRIVE:     u8 = 0x02; // 0–2, see Overdrive enum
    pub const GAME_COLOR:    u8 = 0x0E; // 0–100

    // Colour channels (bonus)
    pub const RED_GAIN:      u8 = 0x16; // 0–100
    pub const GREEN_GAIN:    u8 = 0x18; // 0–100
    pub const BLUE_GAIN:     u8 = 0x1A; // 0–100
}

// ---------------------------------------------------------------------------
// Enums — array-type settings
// ---------------------------------------------------------------------------

/// Maps to VCP 0x14, max value 11.
/// We'll test values 0–11 to find which preset is which.
/// Current value on your monitor: 5
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColorPreset {
    Preset0  = 0,
    Preset1  = 1,
    Preset2  = 2,
    Preset3  = 3,
    Preset4  = 4,
    Preset5  = 5,  // current value on your screen
    Preset6  = 6,
    Preset7  = 7,
    Preset8  = 8,
    Preset9  = 9,
    Preset10 = 10,
    Preset11 = 11,
}

impl ColorPreset {
    pub fn all() -> &'static [ColorPreset] {
        &[
            ColorPreset::Preset0,  ColorPreset::Preset1,
            ColorPreset::Preset2,  ColorPreset::Preset3,
            ColorPreset::Preset4,  ColorPreset::Preset5,
            ColorPreset::Preset6,  ColorPreset::Preset7,
            ColorPreset::Preset8,  ColorPreset::Preset9,
            ColorPreset::Preset10, ColorPreset::Preset11,
        ]
    }

    // Labels are placeholder until we test each value on the physical screen.
    // We'll update these after the calibration step.
    pub fn label(&self) -> &'static str {
        match self {
            ColorPreset::Preset0  => "Warm (test)",
            ColorPreset::Preset1  => "Preset 1 (test)",
            ColorPreset::Preset2  => "Preset 2 (test)",
            ColorPreset::Preset3  => "Preset 3 (test)",
            ColorPreset::Preset4  => "Preset 4 (test)",
            ColorPreset::Preset5  => "Preset 5 (current)",
            ColorPreset::Preset6  => "sRGB (test)",
            ColorPreset::Preset7  => "Preset 7 (test)",
            ColorPreset::Preset8  => "Cool (test)",
            ColorPreset::Preset9  => "Preset 9 (test)",
            ColorPreset::Preset10 => "Preset 10 (test)",
            ColorPreset::Preset11 => "Preset 11 (test)",
        }
    }

    pub fn from_raw(v: u16) -> Option<Self> {
        match v {
            0  => Some(ColorPreset::Preset0),
            1  => Some(ColorPreset::Preset1),
            2  => Some(ColorPreset::Preset2),
            3  => Some(ColorPreset::Preset3),
            4  => Some(ColorPreset::Preset4),
            5  => Some(ColorPreset::Preset5),
            6  => Some(ColorPreset::Preset6),
            7  => Some(ColorPreset::Preset7),
            8  => Some(ColorPreset::Preset8),
            9  => Some(ColorPreset::Preset9),
            10 => Some(ColorPreset::Preset10),
            11 => Some(ColorPreset::Preset11),
            _  => None,
        }
    }
}

/// Maps to VCP 0x02, max value 2 on your monitor.
/// We have 3 steps — we'll label them conservatively until tested.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Overdrive {
    Off    = 0,
    Medium = 1,
    Strong = 2,
}

impl Overdrive {
    pub fn all() -> &'static [Overdrive] {
        &[Overdrive::Off, Overdrive::Medium, Overdrive::Strong]
    }
    pub fn label(&self) -> &'static str {
        match self {
            Overdrive::Off    => "Off",
            Overdrive::Medium => "Medium",
            Overdrive::Strong => "Strong",
        }
    }
    pub fn from_raw(v: u16) -> Option<Self> {
        match v {
            0 => Some(Overdrive::Off),
            1 => Some(Overdrive::Medium),
            2 => Some(Overdrive::Strong),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Settings structs — one per UI tab
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LuminanceSettings {
    pub brightness:   u16,
    pub contrast:     u16,
    pub color_preset: ColorPreset,
    pub dcr:          bool,
    pub hdr_mode:     bool,
}

#[derive(Debug, Clone)]
pub struct GameSettings {
    pub overdrive:  Overdrive,
    pub game_color: u16,         // 0–100
}

#[derive(Debug, Clone)]
pub struct ColorChannels {
    pub red:   u16,  // 0–100
    pub green: u16,  // 0–100
    pub blue:  u16,  // 0–100
}

// ---------------------------------------------------------------------------
// Monitor info — for the selector when you have two screens
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct MonitorInfo {
    pub index:  usize,
    pub model:  String,
    pub serial: String,
}

pub fn enumerate_monitors() -> Vec<MonitorInfo> {
    Display::enumerate()
        .into_iter()
        .enumerate()
        .map(|(index, d)| MonitorInfo {
            index,
            model:  d.info.model_name.clone().unwrap_or_else(|| "Unknown".to_string()),
            serial: d.info.serial_number.clone().unwrap_or_else(|| format!("Display {}", index + 1)),
        })
        .collect()
}

pub fn open_monitor(index: usize) -> Result<Display> {
    Display::enumerate()
        .into_iter()
        .nth(index)
        .context(format!("Monitor {} not found", index))
}

// ---------------------------------------------------------------------------
// Read functions
// ---------------------------------------------------------------------------

// Helper: read a single VCP code, returns the current value
fn read_vcp(display: &mut Display, code: u8) -> Result<u16> {
    Ok(display.handle
        .get_vcp_feature(code)
        .context(format!("Failed to read VCP 0x{:02X}", code))?
        .value())
}

pub fn read_luminance(display: &mut Display) -> Result<LuminanceSettings> {
    Ok(LuminanceSettings {
        brightness:   read_vcp(display, VcpCode::BRIGHTNESS)?,
        contrast:     read_vcp(display, VcpCode::CONTRAST)?,
        color_preset: ColorPreset::from_raw(read_vcp(display, VcpCode::COLOR_PRESET)?)
                          .unwrap_or(ColorPreset::Preset5),
        dcr:          read_vcp(display, VcpCode::DCR)? == 1,
        hdr_mode:     read_vcp(display, VcpCode::HDR_MODE)? == 1,
    })
}

pub fn read_game(display: &mut Display) -> Result<GameSettings> {
    Ok(GameSettings {
        overdrive:  Overdrive::from_raw(read_vcp(display, VcpCode::OVERDRIVE)?)
                        .unwrap_or(Overdrive::Off),
        game_color: read_vcp(display, VcpCode::GAME_COLOR)?,
    })
}

pub fn read_color_channels(display: &mut Display) -> Result<ColorChannels> {
    Ok(ColorChannels {
        red:   read_vcp(display, VcpCode::RED_GAIN)?,
        green: read_vcp(display, VcpCode::GREEN_GAIN)?,
        blue:  read_vcp(display, VcpCode::BLUE_GAIN)?,
    })
}

// ---------------------------------------------------------------------------
// Write functions
// ---------------------------------------------------------------------------

// Helper: write a single VCP code
fn write_vcp(display: &mut Display, code: u8, value: u16) -> Result<()> {
    display.handle
        .set_vcp_feature(code, value)
        .context(format!("Failed to write VCP 0x{:02X} = {}", code, value))
}

pub fn set_brightness(display: &mut Display, v: u16)      -> Result<()> { write_vcp(display, VcpCode::BRIGHTNESS, v) }
pub fn set_contrast(display: &mut Display, v: u16)        -> Result<()> { write_vcp(display, VcpCode::CONTRAST, v) }
pub fn set_color_preset(display: &mut Display, p: ColorPreset) -> Result<()> { write_vcp(display, VcpCode::COLOR_PRESET, p as u16) }
pub fn set_dcr(display: &mut Display, on: bool)           -> Result<()> { write_vcp(display, VcpCode::DCR, on as u16) }
pub fn set_hdr_mode(display: &mut Display, on: bool)      -> Result<()> { write_vcp(display, VcpCode::HDR_MODE, on as u16) }
pub fn set_overdrive(display: &mut Display, o: Overdrive) -> Result<()> { write_vcp(display, VcpCode::OVERDRIVE, o as u16) }
pub fn set_game_color(display: &mut Display, v: u16)      -> Result<()> { write_vcp(display, VcpCode::GAME_COLOR, v) }
pub fn set_red(display: &mut Display, v: u16)             -> Result<()> { write_vcp(display, VcpCode::RED_GAIN, v) }
pub fn set_green(display: &mut Display, v: u16)           -> Result<()> { write_vcp(display, VcpCode::GREEN_GAIN, v) }
pub fn set_blue(display: &mut Display, v: u16)            -> Result<()> { write_vcp(display, VcpCode::BLUE_GAIN, v) }
