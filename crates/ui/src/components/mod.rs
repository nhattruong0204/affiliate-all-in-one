//! Reusable components. All are server-rendered; interactivity (theme, toasts, submit guard)
//! lives in `assets/app.js` and keys off `data-*` attributes rendered here.
//!
//! Styling comes from semantic classes defined in `style/tailwind.css` (`btn`, `input`, `card`,
//! …), so markup stays small and consistent. Colours only come from design tokens.

mod button;
mod card;
mod feedback;
mod form;
mod shell;
mod states;

pub use button::{Button, ButtonLink, ButtonSize, ButtonVariant};
pub use card::{Card, CardPadding};
pub use feedback::{Alert, AlertKind, Toast, ToastKind, ToastRegion};
pub use form::{CheckboxField, InputKind, TextAreaField, TextField};
pub use shell::{AppShell, LanguageSwitch, NavItem, ThemeToggle};
pub use states::{EmptyState, ErrorState, Skeleton, SkeletonGroup, SkeletonText};

/// `None` for an empty string: lets optional props map to omitted attributes.
pub(crate) fn non_empty(s: String) -> Option<String> {
    (!s.is_empty()).then_some(s)
}
