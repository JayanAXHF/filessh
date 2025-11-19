use std::{
    env,
    fmt::{self, Display, Formatter},
    path::PathBuf,
    sync::{LazyLock, OnceLock},
};

use color_eyre::eyre::Result;
use config::{Config, Environment, File};
use rat_theme3::{DarkTheme, Palette, SalsaTheme, ShellTheme};
use ratatui::style::Color;
use serde::Deserialize;

use crate::logging::{PROJECT_NAME, project_directory};

pub static THEME: OnceLock<&'static str> = OnceLock::new();

#[inline(always)]
fn init_theme(name: Box<str>) {
    let static_str: &'static str = Box::leak(name);
    THEME.set(static_str).unwrap();
}

#[inline(always)]
fn current_theme() -> &'static str {
    THEME.get().unwrap()
}

#[derive(Deserialize, Debug, Clone, Default)]
pub(crate) struct Settings {
    #[serde(default)]
    pub(crate) debug: bool,
    #[serde(default)]
    pub(crate) silent: bool,
    #[serde(default)]
    pub(crate) theme: Theme,
}

pub static CONFIG_FOLDER: LazyLock<Option<PathBuf>> = LazyLock::new(|| {
    env::var(format!("{}_CONFIG", &*PROJECT_NAME))
        .ok()
        .map(PathBuf::from)
});

pub(crate) fn get_config_dir() -> PathBuf {
    if let Some(s) = CONFIG_FOLDER.clone() {
        s
    } else if let Some(proj_dirs) = project_directory() {
        proj_dirs.config_local_dir().to_path_buf()
    } else {
        PathBuf::from(".").join(".config")
    }
}

#[derive(Deserialize, Debug, Clone, Copy, Default)]
#[allow(unused)]
pub enum DefaultTheme {
    #[default]
    ImperialDark,
    RadiumDark,
    TundraDark,
    OceanDark,
    MonochromeDark,
    BlackWhiteDark,
    Base16Dark,
    Base16RelaxDark,
    MonekaiDark,
    SolarizedDark,
    OxoCarbonDark,
    RustDark,
    VSCodeDark,
    ImperialShell,
    RadiumShell,
    TundraShell,
    OceanShell,
    MonochromeShell,
    BlackWhiteShell,
    Base16Shell,
    Base16RelaxShell,
    MonekaiShell,
    SolarizedShell,
    OxoCarbonShell,
    RustShell,
    VSCodeShell,
}

#[derive(Deserialize, Debug, Clone)]
#[serde(untagged)]
pub enum Theme {
    Custom(Box<CustomTheme>),
    Default(DefaultTheme),
}

impl Default for Theme {
    fn default() -> Self {
        Self::Default(DefaultTheme::default())
    }
}

impl Display for DefaultTheme {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        type Theme = DefaultTheme;
        let text = match self {
            Theme::ImperialDark => "Imperial Dark",
            Theme::RadiumDark => "Radium Dark",
            Theme::TundraDark => "Tundra Dark",
            Theme::OceanDark => "Ocean Dark",
            Theme::MonochromeDark => "Monochrome Dark",
            Theme::BlackWhiteDark => "Black & White Dark",
            Theme::Base16Dark => "Base16 Dark",
            Theme::Base16RelaxDark => "Base16 Relax Dark",
            Theme::MonekaiDark => "Monekai Dark",
            Theme::SolarizedDark => "Solarized Dark",
            Theme::OxoCarbonDark => "OxoCarbon Dark",
            Theme::RustDark => "Rust Dark",
            Theme::VSCodeDark => "VSCode Dark",
            Theme::ImperialShell => "Imperial Shell",
            Theme::RadiumShell => "Radium Shell",
            Theme::TundraShell => "Tundra Shell",
            Theme::OceanShell => "Ocean Shell",
            Theme::MonochromeShell => "Monochrome Shell",
            Theme::BlackWhiteShell => "Black & White Shell",
            Theme::Base16Shell => "Base16 Shell",
            Theme::Base16RelaxShell => "Base16 Relax Shell",
            Theme::MonekaiShell => "Monekai Shell",
            Theme::SolarizedShell => "Solarized Shell",
            Theme::OxoCarbonShell => "OxoCarbon Shell",
            Theme::RustShell => "Rust Shell",
            Theme::VSCodeShell => "VSCode Shell",
        };

        write!(f, "{}", text)
    }
}

impl Settings {
    pub(crate) fn new() -> Result<Self> {
        let s = Config::builder()
            .add_source(File::from(get_config_dir().join("config")).required(false))
            .add_source(
                Environment::with_prefix(&PROJECT_NAME)
                    .separator("__")
                    .prefix_separator("_"),
            );
        let s = s.build()?;
        Ok(s.try_deserialize().unwrap_or_default())
    }
    pub(crate) fn get_theme(&self) -> &Theme {
        &self.theme
    }
}

pub(crate) struct LoggingConfig {
    silent: bool,
    debug: bool,
}

impl LoggingConfig {
    pub(crate) fn get_level(&self) -> tracing::Level {
        if self.debug {
            tracing::Level::DEBUG
        } else if self.silent {
            tracing::Level::ERROR
        } else {
            tracing::Level::INFO
        }
    }
}

impl From<&Settings> for LoggingConfig {
    fn from(settings: &Settings) -> Self {
        Self {
            silent: settings.silent,
            debug: settings.debug,
        }
    }
}

#[derive(Deserialize, Debug, Clone, Default)]
pub(crate) struct CustomTheme {
    pub(crate) name: Box<str>,
    pub(crate) palette: CustomPalette,
    pub(crate) type_: ThemeType,
}

#[derive(Deserialize, Debug, Clone, Default)]
pub enum ThemeType {
    #[default]
    Dark,
    Shell,
}

#[derive(Debug, Default, Clone, Deserialize)]
pub struct CustomPalette {
    pub name: Box<str>,

    pub text_light: Color,
    pub text_bright: Color,
    pub text_dark: Color,
    pub text_black: Color,

    pub white: [Color; 8],
    pub black: [Color; 8],
    pub gray: [Color; 8],

    pub red: [Color; 8],
    pub orange: [Color; 8],
    pub yellow: [Color; 8],
    pub limegreen: [Color; 8],
    pub green: [Color; 8],
    pub bluegreen: [Color; 8],
    pub cyan: [Color; 8],
    pub blue: [Color; 8],
    pub deepblue: [Color; 8],
    pub purple: [Color; 8],
    pub magenta: [Color; 8],
    pub redpink: [Color; 8],

    pub primary: [Color; 8],
    pub secondary: [Color; 8],
}

impl From<CustomPalette> for Palette {
    fn from(palette: CustomPalette) -> Self {
        init_theme(palette.name);
        Self {
            name: current_theme(),
            text_light: palette.text_light,
            text_bright: palette.text_bright,
            text_dark: palette.text_dark,
            text_black: palette.text_black,
            white: palette.white,
            black: palette.black,
            gray: palette.gray,
            red: palette.red,
            orange: palette.orange,
            yellow: palette.yellow,
            limegreen: palette.limegreen,
            green: palette.green,
            bluegreen: palette.bluegreen,
            cyan: palette.cyan,
            blue: palette.blue,
            deepblue: palette.deepblue,
            purple: palette.purple,
            magenta: palette.magenta,
            redpink: palette.redpink,
            primary: palette.primary,
            secondary: palette.secondary,
        }
    }
}

impl From<CustomTheme> for Box<dyn SalsaTheme> {
    fn from(theme: CustomTheme) -> Self {
        let p = theme.palette.into();
        match theme.type_ {
            ThemeType::Dark => Box::new(DarkTheme::new(&theme.name, p)),
            ThemeType::Shell => Box::new(ShellTheme::new(&theme.name, p)),
        }
    }
}

impl From<Box<CustomTheme>> for Box<dyn SalsaTheme> {
    fn from(theme: Box<CustomTheme>) -> Self {
        let p = theme.palette.into();
        match theme.type_ {
            ThemeType::Dark => Box::new(DarkTheme::new(&theme.name, p)),
            ThemeType::Shell => Box::new(ShellTheme::new(&theme.name, p)),
        }
    }
}

impl From<Box<dyn SalsaTheme>> for CustomTheme {
    fn from(theme: Box<dyn SalsaTheme>) -> Self {
        let p = theme.palette();
        if theme.name().contains("Shell") {
            CustomTheme {
                name: Box::from("Shell"),
                palette: CustomPalette {
                    name: Box::from(p.name),
                    text_light: p.text_light,
                    text_bright: p.text_bright,
                    text_dark: p.text_dark,
                    text_black: p.text_black,
                    white: p.white,
                    black: p.black,
                    gray: p.gray,
                    red: p.red,
                    orange: p.orange,
                    yellow: p.yellow,
                    limegreen: p.limegreen,
                    green: p.green,
                    bluegreen: p.bluegreen,
                    cyan: p.cyan,
                    blue: p.blue,
                    deepblue: p.deepblue,
                    purple: p.purple,
                    magenta: p.magenta,
                    redpink: p.redpink,
                    primary: p.primary,
                    secondary: p.secondary,
                },
                type_: ThemeType::Shell,
            }
        } else {
            CustomTheme {
                name: Box::from("Dark"),
                palette: CustomPalette {
                    name: Box::from(p.name),
                    text_light: p.text_light,
                    text_bright: p.text_bright,
                    text_dark: p.text_dark,
                    text_black: p.text_black,
                    white: p.white,
                    black: p.black,
                    gray: p.gray,
                    red: p.red,
                    orange: p.orange,
                    yellow: p.yellow,
                    limegreen: p.limegreen,
                    green: p.green,
                    bluegreen: p.bluegreen,
                    cyan: p.cyan,
                    blue: p.blue,
                    deepblue: p.deepblue,
                    purple: p.purple,
                    magenta: p.magenta,
                    redpink: p.redpink,
                    primary: p.primary,
                    secondary: p.secondary,
                },
                type_: ThemeType::Dark,
            }
        }
    }
}

pub fn install_manpages() -> Result<()> {
    use std::{env, fs, path::PathBuf};
    // Windows: silently skip installation
    #[cfg(target_os = "windows")]
    {
        eprintln!("Manpage installation is not supported on Windows.");
        return Ok(());
    }

    // Unix-like systems: install normally
    #[cfg(not(target_os = "windows"))]
    {
        let prefix = env::var("PREFIX").unwrap_or("/usr/local".to_string());

        let man1_dir = PathBuf::from(&prefix).join("share/man/man1");
        let man5_dir = PathBuf::from(&prefix).join("share/man/man5");

        fs::create_dir_all(&man1_dir)?;
        fs::create_dir_all(&man5_dir)?;

        // Build file names dynamically
        let man1_file = format!("{}.1", &*PROJECT_NAME).to_lowercase();
        let man5_file = format!("{}.5", &*PROJECT_NAME).to_lowercase();

        // Embed manpages
        // Adjust the include paths to your repo structure
        let man1_contents = include_str!(concat!(env!("OUT_DIR"), "/filessh.1"));
        let man5_contents = include_str!("../man/filessh.5");

        // Write them to the correct directories
        fs::write(man1_dir.join(&man1_file), man1_contents)?;
        fs::write(man5_dir.join(&man5_file), man5_contents)?;

        println!("Installed manpages:");
        println!("  {}/share/man/man1/{}", prefix, man1_file);
        println!("  {}/share/man/man5/{}", prefix, man5_file);

        Ok(())
    }
}
