use colored::Color;

/// Color theme for terminal output
pub struct Theme {
    // Base colors
    pub base: Color,
    pub text: Color,
    pub subtext: Color,
    
    // Status colors
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
    pub blue: Color,
    pub mauve: Color,
    pub teal: Color,
    
    // Grays
    pub surface: Color,
    pub overlay: Color,
}

/// Catppuccin Mocha theme
pub const CATPPUCCIN: Theme = Theme {
    base: Color::TrueColor { r: 30, g: 30, b: 46 },      // #1e1e2e
    text: Color::TrueColor { r: 205, g: 214, b: 244 },   // #cdd6f4
    subtext: Color::TrueColor { r: 166, g: 173, b: 200 }, // #a6adc8
    
    green: Color::TrueColor { r: 166, g: 227, b: 161 },   // #a6e3a1
    yellow: Color::TrueColor { r: 249, g: 226, b: 175 },  // #f9e2af
    red: Color::TrueColor { r: 243, g: 139, b: 168 },     // #f38ba8
    blue: Color::TrueColor { r: 137, g: 180, b: 250 },    // #89b4fa
    mauve: Color::TrueColor { r: 203, g: 166, b: 247 },   // #cba6f7
    teal: Color::TrueColor { r: 148, g: 226, b: 213 },    // #94e2d5
    
    surface: Color::TrueColor { r: 49, g: 50, b: 68 },    // #313244
    overlay: Color::TrueColor { r: 108, g: 112, b: 134 }, // #6c7086
};

/// Nord theme
pub const NORD: Theme = Theme {
    base: Color::TrueColor { r: 46, g: 52, b: 64 },       // #2e3440
    text: Color::TrueColor { r: 236, g: 239, b: 244 },    // #eceff4
    subtext: Color::TrueColor { r: 216, g: 222, b: 233 }, // #d8dee9
    
    green: Color::TrueColor { r: 163, g: 190, b: 140 },   // #a3be8c
    yellow: Color::TrueColor { r: 235, g: 203, b: 139 },  // #ebcb8b
    red: Color::TrueColor { r: 191, g: 97, b: 106 },      // #bf616a
    blue: Color::TrueColor { r: 129, g: 161, b: 193 },    // #81a1c1
    mauve: Color::TrueColor { r: 180, g: 142, b: 173 },   // #b48ead
    teal: Color::TrueColor { r: 136, g: 192, b: 208 },    // #88c0d0
    
    surface: Color::TrueColor { r: 59, g: 66, b: 82 },    // #3b4252
    overlay: Color::TrueColor { r: 76, g: 86, b: 106 },   // #4c566a
};

/// Dracula theme
pub const DRACULA: Theme = Theme {
    base: Color::TrueColor { r: 40, g: 42, b: 54 },       // #282a36
    text: Color::TrueColor { r: 248, g: 248, b: 242 },    // #f8f8f2
    subtext: Color::TrueColor { r: 98, g: 114, b: 164 },  // #6272a4
    
    green: Color::TrueColor { r: 80, g: 250, b: 123 },    // #50fa7b
    yellow: Color::TrueColor { r: 241, g: 250, b: 140 },  // #f1fa8c
    red: Color::TrueColor { r: 255, g: 85, b: 85 },       // #ff5555
    blue: Color::TrueColor { r: 139, g: 233, b: 253 },    // #8be9fd
    mauve: Color::TrueColor { r: 189, g: 147, b: 249 },   // #bd93f9
    teal: Color::TrueColor { r: 139, g: 233, b: 253 },    // #8be9fd
    
    surface: Color::TrueColor { r: 68, g: 71, b: 90 },    // #44475a
    overlay: Color::TrueColor { r: 98, g: 114, b: 164 },  // #6272a4
};

/// Default theme (uses terminal colors)
pub const DEFAULT: Theme = Theme {
    base: Color::Black,
    text: Color::White,
    subtext: Color::BrightBlack,
    
    green: Color::Green,
    yellow: Color::Yellow,
    red: Color::Red,
    blue: Color::Blue,
    mauve: Color::Magenta,
    teal: Color::Cyan,
    
    surface: Color::Black,
    overlay: Color::BrightBlack,
};

pub fn get_theme(name: &str) -> &'static Theme {
    match name {
        "catppuccin" => &CATPPUCCIN,
        "nord" => &NORD,
        "dracula" => &DRACULA,
        _ => &DEFAULT,
    }
}
