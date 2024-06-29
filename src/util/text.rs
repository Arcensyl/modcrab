//! This module provides facilities for coloring and styling strings.

// Source for ANSI codes: https://gist.github.com/fnky/458719343aabd01cfb17a3a4f7296797

/// A trait to provide text styling capability to strings.
pub trait FancyText {
	/// Stylize a string with the provided options.
	///
	/// # Parameters
	/// *style* - The text style to use, such as bold or italic.
	/// *foreground* - The color of the text itself.
	/// *background* - The color of the text's background.
    fn stylize(
        &self,
        style: Option<TextStyle>,
        foreground: Option<TextColor>,
        background: Option<TextColor>,
    ) -> String;
}

/// The ANSI escape code to reset all styles and colors.
const ANSI_RESET: &'static str = "\x1B[0m";

/// Various styles of text.
/// This enum is not exhaustive; it only has styles I care about.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextStyle {
    /// **Bold** text.
    Bold,

    /// *Italic* text.
    Italic,

    /// Underlined text.
    Underlined,

    /// Strikedthrough text.
    Strikedthrough,
}

/// Various text colors available in the terminal.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TextColor {
    True(u8, u8, u8), // RGB
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl FancyText for str {
    fn stylize(
        &self,
        style: Option<TextStyle>,
        foreground: Option<TextColor>,
        background: Option<TextColor>,
    ) -> String {
        let style_code = match style {
			Some(style) => style.ansi(),
			None => String::new(),
		};

		let fg_code = match foreground {
			Some(fg) => fg.ansi_fg(),
			None => String::new(),
		};

        let bg_code = match background {
			Some(bg) => bg.ansi_bg(),
			None => String::new(),
		};

		// Returns the provided string wrapped in the relevant ANSI codes.
		format!("{style_code}{fg_code}{bg_code}{self}{ANSI_RESET}")
    }
}

impl TextStyle {
	/// Returns this style's associated ANSI escape code.
    pub fn ansi(&self) -> String {
        let code = match self {
            TextStyle::Bold => "\x1B[1m",
            TextStyle::Italic => "\x1B[3m",
            TextStyle::Underlined => "\x1B[4m",
            TextStyle::Strikedthrough => "\x1B[9m",
        };

		code.to_owned()
    }
}

impl TextColor {
	/// Returns this color's associated ANSI escape code.
	/// This is specifically the code for changing the text's foreground color.
    pub fn ansi_fg(&self) -> String {
		let code = match self {
            TextColor::True(r, g, b) => return format!("\x1B[38;2;{r};{g};{b}m"),
            TextColor::Black => "\x1B[30m",
            TextColor::Red => "\x1B[31m",
            TextColor::Green => "\x1B[32m",
            TextColor::Yellow => "\x1B[33m",
            TextColor::Blue => "\x1B[34m",
            TextColor::Magenta => "\x1B[35m",
            TextColor::Cyan => "\x1B[36m",
            TextColor::White => "\x1B[37m",
        };

		code.to_owned()
    }

	/// Returns this color's associated ANSI escape code.
	/// This is specifically the code for changing the text's background color.
    pub fn ansi_bg(&self) -> String {
		let code = match self {
            TextColor::True(r, g, b) => return format!("\x1B[48;2;{r};{g};{b}m"),
            TextColor::Black => "\x1B[40m",
            TextColor::Red => "\x1B[41m",
            TextColor::Green => "\x1B[42m",
            TextColor::Yellow => "\x1B[43m",
            TextColor::Blue => "\x1B[44m",
            TextColor::Magenta => "\x1B[45m",
            TextColor::Cyan => "\x1B[46m",
            TextColor::White => "\x1B[47m",
        };

		code.to_owned()
    }
}
