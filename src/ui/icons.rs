/// Icon set for terminal output
pub struct IconSet {
    // Stack elements
    pub working: &'static str,
    pub change: &'static str,
    pub main: &'static str,
    
    // Connections
    pub pipe: &'static str,
    pub branch: &'static str,
    pub last: &'static str,
    
    // Status indicators
    pub bookmark: &'static str,
    pub pr_open: &'static str,
    pub pr_approved: &'static str,
    pub pr_merged: &'static str,
    pub ci_running: &'static str,
    pub ci_passed: &'static str,
    pub ci_failed: &'static str,
    
    // Actions
    pub ready: &'static str,
    pub waiting: &'static str,
    pub blocked: &'static str,
    
    // Suggestions
    pub lightbulb: &'static str,
    pub warning: &'static str,
    pub error: &'static str,
    pub info: &'static str,
}

pub const UNICODE_ICONS: IconSet = IconSet {
    // Stack elements
    working: "●",
    change: "○",
    main: "◆",
    
    // Connections
    pipe: "│",
    branch: "├",
    last: "└",
    
    // Status indicators
    bookmark: "→",
    pr_open: "◈",
    pr_approved: "✓",
    pr_merged: "✔",
    ci_running: "⟳",
    ci_passed: "✓",
    ci_failed: "✗",
    
    // Actions
    ready: "◉",
    waiting: "◎",
    blocked: "◌",
    
    // Suggestions
    lightbulb: "💡",
    warning: "⚠",
    error: "✗",
    info: "ℹ",
};

pub const ASCII_ICONS: IconSet = IconSet {
    // Stack elements
    working: "*",
    change: "o",
    main: "#",

    // Connections
    pipe: "|",
    branch: "+",
    last: "\\",

    // Status indicators
    bookmark: "->",
    pr_open: "PR",
    pr_approved: "OK",
    pr_merged: "++",
    ci_running: "~~",
    ci_passed: "OK",
    ci_failed: "XX",

    // Actions
    ready: "!",
    waiting: "...",
    blocked: "X",

    // Suggestions
    lightbulb: "!",
    warning: "!",
    error: "X",
    info: "i",
};

pub const NERDFONT_ICONS: IconSet = IconSet {
    // Stack elements (git icons)
    working: "\u{e725}",  //  git branch
    change: "\u{e729}",   //  git commit
    main: "\u{e727}",     //  git merge

    // Connections
    pipe: "│",
    branch: "├",
    last: "└",

    // Status indicators
    bookmark: "\u{f02e}",      //  bookmark
    pr_open: "\u{f407}",       //  pull request
    pr_approved: "\u{f058}",   //  check circle
    pr_merged: "\u{e727}",     //  git merge
    ci_running: "\u{f021}",    //  sync/refresh
    ci_passed: "\u{f00c}",     //  check
    ci_failed: "\u{f00d}",     //  times

    // Actions
    ready: "\u{f058}",    //  check circle
    waiting: "\u{f017}",  //  clock
    blocked: "\u{f057}",  //  times circle

    // Suggestions
    lightbulb: "\u{f0eb}",  //  lightbulb
    warning: "\u{f071}",    //  warning triangle
    error: "\u{f057}",      //  times circle
    info: "\u{f05a}",       //  info circle
};

pub fn get_icon_set(style: &str) -> &'static IconSet {
    match style {
        "ascii" => &ASCII_ICONS,
        "nerdfont" | "nerd" => &NERDFONT_ICONS,
        _ => &UNICODE_ICONS,
    }
}
