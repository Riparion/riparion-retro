//! Shared class-string constants so the CRT look stays consistent.

/// Standard touch button: phosphor border, ≥44px target, inverts when pressed.
pub const BTN: &str = "crt-btn";
/// Full-width variant for stacked menus.
pub const BTN_WIDE: &str = "crt-btn w-full";
/// Emphasized (filled) variant for primary/confirm actions.
pub const BTN_PRIMARY: &str = "crt-btn crt-btn-primary w-full";
/// Bordered content panel.
pub const PANEL: &str = "crt-panel";
/// Sticky bottom action bar with safe-area padding.
pub const ACTION_BAR: &str = "action-bar";

/// A standard screen body: a centered, max-width column with page padding.
/// Screens append their own `gap-N`. Use [`SCREEN_CENTERED`] to also center
/// vertically, or [`SCREEN_HERO`] for a full-width centered title/end screen.
pub const SCREEN: &str = "flex-1 flex flex-col p-4 max-w-md w-full mx-auto";
/// [`SCREEN`] with its content centered vertically.
pub const SCREEN_CENTERED: &str = "flex-1 flex flex-col justify-center p-4 max-w-md w-full mx-auto";
/// A full-width, fully-centered hero layout (title and game-over screens);
/// screens append their own `gap-N`/`p-N`.
pub const SCREEN_HERO: &str = "flex-1 flex flex-col items-center justify-center text-center";
