//! Settings panel state management.
//!
//! This module provides state management for the settings UI, including
//! section navigation, item selection, and edit mode handling.

use whip_config::{Config, Repository};

/// Sections in the settings panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsSection {
    /// Repository list section.
    #[default]
    Repositories,
    /// Polling configuration section.
    Polling,
    /// Authentication section.
    Authentication,
}

impl SettingsSection {
    /// Returns the next section (wrapping around).
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Repositories => Self::Polling,
            Self::Polling => Self::Authentication,
            Self::Authentication => Self::Repositories,
        }
    }

    /// Returns the previous section (wrapping around).
    #[must_use]
    pub fn prev(self) -> Self {
        match self {
            Self::Repositories => Self::Authentication,
            Self::Polling => Self::Repositories,
            Self::Authentication => Self::Polling,
        }
    }

    /// Returns the display name for this section.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Repositories => "Repositories",
            Self::Polling => "Polling",
            Self::Authentication => "Authentication",
        }
    }

    /// Returns all sections in order.
    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::Repositories, Self::Polling, Self::Authentication]
    }
}

/// Edit mode for settings fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum EditMode {
    /// Not currently editing.
    #[default]
    None,
    /// Editing a text field.
    Text {
        /// The current value being edited.
        value: String,
        /// The cursor position within the value.
        cursor: usize,
    },
    /// Adding a new repository.
    AddRepository {
        /// The input value (in "owner/repo" format).
        value: String,
        /// The cursor position.
        cursor: usize,
    },
}

impl EditMode {
    /// Returns `true` if currently in edit mode.
    #[must_use]
    pub fn is_editing(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Returns the current edit value, if any.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Text { value, .. } | Self::AddRepository { value, .. } => Some(value),
        }
    }

    /// Returns the cursor position, if in edit mode.
    #[must_use]
    pub fn cursor(&self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Text { cursor, .. } | Self::AddRepository { cursor, .. } => Some(*cursor),
        }
    }

    /// Inserts a character at the cursor position.
    pub fn insert_char(&mut self, ch: char) {
        match self {
            Self::None => {}
            Self::Text { value, cursor } | Self::AddRepository { value, cursor } => {
                value.insert(*cursor, ch);
                *cursor += ch.len_utf8();
            }
        }
    }

    /// Deletes the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        match self {
            Self::None => {}
            Self::Text { value, cursor } | Self::AddRepository { value, cursor } => {
                if *cursor > 0 {
                    // Find the previous character boundary
                    let prev_boundary = value[..*cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    value.remove(prev_boundary);
                    *cursor = prev_boundary;
                }
            }
        }
    }
}

/// State for the settings panel.
#[derive(Debug, Clone)]
pub struct SettingsState {
    /// The configuration being edited.
    config: Config,
    /// Whether the config has unsaved changes.
    dirty: bool,
    /// The currently selected section.
    section: SettingsSection,
    /// The selected item index within the current section.
    selected_item: usize,
    /// The current edit mode.
    edit_mode: EditMode,
}

impl SettingsState {
    /// Creates a new settings state from a configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - The configuration to edit
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_config::Config;
    /// use whip_tui::settings_state::SettingsState;
    ///
    /// let config = Config::default();
    /// let state = SettingsState::new(config);
    /// assert!(!state.is_dirty());
    /// ```
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            dirty: false,
            section: SettingsSection::default(),
            selected_item: 0,
            edit_mode: EditMode::None,
        }
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a mutable reference to the configuration.
    ///
    /// Note: This does not automatically mark the config as dirty.
    /// Call `mark_dirty()` after making changes.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    /// Returns whether the configuration has unsaved changes.
    #[must_use]
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Marks the configuration as having unsaved changes.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Marks the configuration as saved (no longer dirty).
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Returns the currently selected section.
    #[must_use]
    pub fn section(&self) -> SettingsSection {
        self.section
    }

    /// Returns the selected item index within the current section.
    #[must_use]
    pub fn selected_item(&self) -> usize {
        self.selected_item
    }

    /// Returns a reference to the current edit mode.
    #[must_use]
    pub fn edit_mode(&self) -> &EditMode {
        &self.edit_mode
    }

    /// Returns `true` if currently in edit mode.
    #[must_use]
    pub fn is_editing(&self) -> bool {
        self.edit_mode.is_editing()
    }

    /// Moves to the next section.
    pub fn next_section(&mut self) {
        self.section = self.section.next();
        self.selected_item = 0;
    }

    /// Moves to the previous section.
    pub fn prev_section(&mut self) {
        self.section = self.section.prev();
        self.selected_item = 0;
    }

    /// Navigates within the current section.
    ///
    /// # Arguments
    ///
    /// * `delta` - Direction to navigate (positive = down, negative = up)
    pub fn navigate(&mut self, delta: i32) {
        let item_count = self.item_count();
        if item_count == 0 {
            self.selected_item = 0;
            return;
        }

        let new_idx = if delta > 0 {
            (self.selected_item + delta as usize) % item_count
        } else {
            let abs_delta = delta.unsigned_abs() as usize;
            if abs_delta > self.selected_item {
                item_count - ((abs_delta - self.selected_item) % item_count)
            } else {
                self.selected_item - abs_delta
            }
        };
        self.selected_item = new_idx.min(item_count.saturating_sub(1));
    }

    /// Returns the number of items in the current section.
    #[must_use]
    pub fn item_count(&self) -> usize {
        match self.section {
            // Repositories + "Add new" option
            SettingsSection::Repositories => self.config.repositories.len() + 1,
            // Polling interval, auto-adjust toggle
            SettingsSection::Polling => 2,
            // Global token field
            SettingsSection::Authentication => 1,
        }
    }

    /// Starts editing the currently selected item.
    pub fn start_edit(&mut self) {
        match self.section {
            SettingsSection::Repositories => {
                if self.selected_item < self.config.repositories.len() {
                    // Edit existing repository (not implemented - show read-only)
                } else {
                    // Add new repository
                    self.edit_mode = EditMode::AddRepository {
                        value: String::new(),
                        cursor: 0,
                    };
                }
            }
            SettingsSection::Polling => {
                if self.selected_item == 0 {
                    // Edit polling interval
                    self.edit_mode = EditMode::Text {
                        value: self.config.polling.interval_secs.to_string(),
                        cursor: self.config.polling.interval_secs.to_string().len(),
                    };
                }
                // Auto-adjust is a toggle, handled differently
            }
            SettingsSection::Authentication => {
                // Edit global token
                self.edit_mode = EditMode::Text {
                    value: self.config.github_token.clone().unwrap_or_default(),
                    cursor: self.config.github_token.as_ref().map_or(0, |t| t.len()),
                };
            }
        }
    }

    /// Confirms the current edit.
    pub fn confirm_edit(&mut self) {
        match &self.edit_mode {
            EditMode::None => {}
            EditMode::Text { value, .. } => {
                match self.section {
                    SettingsSection::Polling if self.selected_item == 0 => {
                        // Parse and set polling interval
                        if let Ok(interval) = value.parse::<u32>() {
                            self.config.polling.interval_secs = interval;
                            self.dirty = true;
                        }
                    }
                    SettingsSection::Authentication => {
                        // Set global token
                        self.config.github_token = if value.is_empty() {
                            None
                        } else {
                            Some(value.clone())
                        };
                        self.dirty = true;
                    }
                    _ => {}
                }
                self.edit_mode = EditMode::None;
            }
            EditMode::AddRepository { value, .. } => {
                // Try to parse and add the repository
                if let Ok(repo) = Repository::parse_short(value) {
                    self.config.repositories.push(repo);
                    self.dirty = true;
                }
                self.edit_mode = EditMode::None;
            }
        }
    }

    /// Cancels the current edit.
    pub fn cancel_edit(&mut self) {
        self.edit_mode = EditMode::None;
    }

    /// Deletes the currently selected item (if applicable).
    ///
    /// Returns `true` if an item was deleted.
    #[must_use]
    pub fn delete_selected(&mut self) -> bool {
        match self.section {
            SettingsSection::Repositories => {
                if self.selected_item < self.config.repositories.len() {
                    self.config.repositories.remove(self.selected_item);
                    self.dirty = true;
                    // Adjust selection if needed
                    if self.selected_item >= self.config.repositories.len()
                        && self.selected_item > 0
                    {
                        self.selected_item -= 1;
                    }
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Toggles boolean settings (like auto-adjust).
    pub fn toggle_selected(&mut self) {
        match self.section {
            SettingsSection::Polling if self.selected_item == 1 => {
                self.config.polling.auto_adjust = !self.config.polling.auto_adjust;
                self.dirty = true;
            }
            _ => {}
        }
    }

    /// Handles character input while in edit mode.
    pub fn input_char(&mut self, ch: char) {
        self.edit_mode.insert_char(ch);
    }

    /// Handles backspace while in edit mode.
    pub fn backspace(&mut self) {
        self.edit_mode.backspace();
    }

    /// Takes the configuration out of this state, consuming it.
    #[must_use]
    pub fn into_config(self) -> Config {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_section_navigation() {
        let mut section = SettingsSection::Repositories;
        section = section.next();
        assert_eq!(section, SettingsSection::Polling);
        section = section.next();
        assert_eq!(section, SettingsSection::Authentication);
        section = section.next();
        assert_eq!(section, SettingsSection::Repositories);

        section = section.prev();
        assert_eq!(section, SettingsSection::Authentication);
    }

    #[test]
    fn settings_section_names() {
        assert_eq!(SettingsSection::Repositories.name(), "Repositories");
        assert_eq!(SettingsSection::Polling.name(), "Polling");
        assert_eq!(SettingsSection::Authentication.name(), "Authentication");
    }

    #[test]
    fn settings_state_new() {
        let config = Config::default();
        let state = SettingsState::new(config);
        assert!(!state.is_dirty());
        assert_eq!(state.section(), SettingsSection::Repositories);
        assert_eq!(state.selected_item(), 0);
        assert!(!state.is_editing());
    }

    #[test]
    fn settings_state_section_navigation() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        state.next_section();
        assert_eq!(state.section(), SettingsSection::Polling);

        state.prev_section();
        assert_eq!(state.section(), SettingsSection::Repositories);
    }

    #[test]
    fn settings_state_item_count() {
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner", "repo"));

        let mut state = SettingsState::new(config);

        // Repositories: 1 repo + "Add new"
        assert_eq!(state.item_count(), 2);

        state.next_section();
        // Polling: interval + auto-adjust
        assert_eq!(state.item_count(), 2);

        state.next_section();
        // Authentication: global token
        assert_eq!(state.item_count(), 1);
    }

    #[test]
    fn settings_state_navigate() {
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner1", "repo1"));
        config.repositories.push(Repository::new("owner2", "repo2"));

        let mut state = SettingsState::new(config);

        // 3 items: 2 repos + "Add new"
        state.navigate(1);
        assert_eq!(state.selected_item(), 1);

        state.navigate(1);
        assert_eq!(state.selected_item(), 2);

        // Wrap around
        state.navigate(1);
        assert_eq!(state.selected_item(), 0);

        // Navigate up wraps
        state.navigate(-1);
        assert_eq!(state.selected_item(), 2);
    }

    #[test]
    fn edit_mode_text_input() {
        let mut edit = EditMode::Text {
            value: String::new(),
            cursor: 0,
        };

        edit.insert_char('h');
        edit.insert_char('i');
        assert_eq!(edit.value(), Some("hi"));
        assert_eq!(edit.cursor(), Some(2));

        edit.backspace();
        assert_eq!(edit.value(), Some("h"));
        assert_eq!(edit.cursor(), Some(1));
    }

    #[test]
    fn settings_state_add_repository() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        // Select "Add new" (index 0 since no repos)
        state.start_edit();
        assert!(state.is_editing());

        state.input_char('o');
        state.input_char('w');
        state.input_char('n');
        state.input_char('e');
        state.input_char('r');
        state.input_char('/');
        state.input_char('r');
        state.input_char('e');
        state.input_char('p');
        state.input_char('o');

        state.confirm_edit();
        assert!(!state.is_editing());
        assert!(state.is_dirty());
        assert_eq!(state.config().repositories.len(), 1);
        assert_eq!(state.config().repositories[0].full_name(), "owner/repo");
    }

    #[test]
    fn settings_state_delete_repository() {
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner", "repo"));

        let mut state = SettingsState::new(config);

        assert!(state.delete_selected());
        assert!(state.is_dirty());
        assert!(state.config().repositories.is_empty());
    }

    #[test]
    fn settings_state_toggle_auto_adjust() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        state.next_section(); // Go to Polling
        state.navigate(1); // Select auto-adjust (index 1)

        let initial = state.config().polling.auto_adjust;
        state.toggle_selected();
        assert_eq!(state.config().polling.auto_adjust, !initial);
        assert!(state.is_dirty());
    }

    #[test]
    fn settings_state_edit_polling_interval() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        state.next_section(); // Go to Polling

        state.start_edit();
        assert!(state.is_editing());

        // Clear and type new value
        while state.edit_mode().value().is_some_and(|v| !v.is_empty()) {
            state.backspace();
        }
        state.input_char('1');
        state.input_char('2');
        state.input_char('0');

        state.confirm_edit();
        assert_eq!(state.config().polling.interval_secs, 120);
        assert!(state.is_dirty());
    }

    #[test]
    fn settings_state_cancel_edit() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        state.start_edit();
        state.input_char('x');
        state.cancel_edit();

        assert!(!state.is_editing());
        assert!(!state.is_dirty());
    }
}
