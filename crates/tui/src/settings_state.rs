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

/// Which field is being edited in repository edit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RepoEditField {
    /// Editing the owner/repo path.
    #[default]
    Path,
    /// Editing the optional token.
    Token,
}

impl RepoEditField {
    /// Switches to the next field.
    #[must_use]
    pub fn next(self) -> Self {
        match self {
            Self::Path => Self::Token,
            Self::Token => Self::Path,
        }
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
    /// Editing an existing repository.
    EditRepository {
        /// Index of the repository being edited.
        index: usize,
        /// The repository path (owner/repo).
        path: String,
        /// Cursor position in the path field.
        path_cursor: usize,
        /// The optional token.
        token: String,
        /// Cursor position in the token field.
        token_cursor: usize,
        /// Which field is currently active.
        active_field: RepoEditField,
    },
}

impl EditMode {
    /// Returns `true` if currently in edit mode.
    #[must_use]
    pub fn is_editing(&self) -> bool {
        !matches!(self, Self::None)
    }

    /// Returns the current edit value, if any.
    ///
    /// For `EditRepository`, returns the active field's value.
    #[must_use]
    pub fn value(&self) -> Option<&str> {
        match self {
            Self::None => None,
            Self::Text { value, .. } | Self::AddRepository { value, .. } => Some(value),
            Self::EditRepository {
                path,
                token,
                active_field,
                ..
            } => Some(match active_field {
                RepoEditField::Path => path,
                RepoEditField::Token => token,
            }),
        }
    }

    /// Returns the cursor position, if in edit mode.
    #[must_use]
    pub fn cursor(&self) -> Option<usize> {
        match self {
            Self::None => None,
            Self::Text { cursor, .. } | Self::AddRepository { cursor, .. } => Some(*cursor),
            Self::EditRepository {
                path_cursor,
                token_cursor,
                active_field,
                ..
            } => Some(match active_field {
                RepoEditField::Path => *path_cursor,
                RepoEditField::Token => *token_cursor,
            }),
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
            Self::EditRepository {
                path,
                path_cursor,
                token,
                token_cursor,
                active_field,
                ..
            } => match active_field {
                RepoEditField::Path => {
                    path.insert(*path_cursor, ch);
                    *path_cursor += ch.len_utf8();
                }
                RepoEditField::Token => {
                    token.insert(*token_cursor, ch);
                    *token_cursor += ch.len_utf8();
                }
            },
        }
    }

    /// Deletes the character before the cursor (backspace).
    pub fn backspace(&mut self) {
        fn do_backspace(value: &mut String, cursor: &mut usize) {
            if *cursor > 0 {
                let prev_boundary = value[..*cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                value.remove(prev_boundary);
                *cursor = prev_boundary;
            }
        }

        match self {
            Self::None => {}
            Self::Text { value, cursor } | Self::AddRepository { value, cursor } => {
                do_backspace(value, cursor);
            }
            Self::EditRepository {
                path,
                path_cursor,
                token,
                token_cursor,
                active_field,
                ..
            } => match active_field {
                RepoEditField::Path => do_backspace(path, path_cursor),
                RepoEditField::Token => do_backspace(token, token_cursor),
            },
        }
    }

    /// Moves the cursor one character to the left (respecting UTF-8 boundaries).
    ///
    /// Does nothing if the cursor is already at position 0 or not in edit mode.
    pub fn move_cursor_left(&mut self) {
        fn do_move_left(value: &str, cursor: &mut usize) {
            if *cursor > 0 {
                // Find the previous character boundary
                *cursor = value[..*cursor]
                    .char_indices()
                    .last()
                    .map(|(i, _)| i)
                    .unwrap_or(0);
            }
        }

        match self {
            Self::None => {}
            Self::Text { value, cursor } | Self::AddRepository { value, cursor } => {
                do_move_left(value, cursor);
            }
            Self::EditRepository {
                path,
                path_cursor,
                token,
                token_cursor,
                active_field,
                ..
            } => match active_field {
                RepoEditField::Path => do_move_left(path, path_cursor),
                RepoEditField::Token => do_move_left(token, token_cursor),
            },
        }
    }

    /// Moves the cursor one character to the right (respecting UTF-8 boundaries).
    ///
    /// Does nothing if the cursor is already at the end of the text or not in edit mode.
    pub fn move_cursor_right(&mut self) {
        fn do_move_right(value: &str, cursor: &mut usize) {
            if *cursor < value.len() {
                // Find the next character boundary
                *cursor = value[*cursor..]
                    .char_indices()
                    .nth(1)
                    .map(|(i, _)| *cursor + i)
                    .unwrap_or(value.len());
            }
        }

        match self {
            Self::None => {}
            Self::Text { value, cursor } | Self::AddRepository { value, cursor } => {
                do_move_right(value, cursor);
            }
            Self::EditRepository {
                path,
                path_cursor,
                token,
                token_cursor,
                active_field,
                ..
            } => match active_field {
                RepoEditField::Path => do_move_right(path, path_cursor),
                RepoEditField::Token => do_move_right(token, token_cursor),
            },
        }
    }

    /// Switches to the next field in repository edit mode.
    ///
    /// Does nothing if not in `EditRepository` mode.
    pub fn switch_field(&mut self) {
        if let Self::EditRepository { active_field, .. } = self {
            *active_field = active_field.next();
        }
    }

    /// Returns the active field in repository edit mode, if applicable.
    #[must_use]
    pub fn active_repo_field(&self) -> Option<RepoEditField> {
        if let Self::EditRepository { active_field, .. } = self {
            Some(*active_field)
        } else {
            None
        }
    }

    /// Returns repository edit data if in that mode.
    #[must_use]
    pub fn repo_edit_data(&self) -> Option<(&str, &str, RepoEditField)> {
        if let Self::EditRepository {
            path,
            token,
            active_field,
            ..
        } = self
        {
            Some((path, token, *active_field))
        } else {
            None
        }
    }
}

/// State for the settings panel.
#[derive(Debug, Clone)]
pub struct SettingsState {
    /// The configuration being edited.
    config: Config,
    /// The currently selected section.
    section: SettingsSection,
    /// The selected item index within the current section.
    selected_item: usize,
    /// The current edit mode.
    edit_mode: EditMode,
    /// Index of item pending deletion (waiting for confirmation).
    pending_delete: Option<usize>,
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
    /// assert_eq!(state.section(), whip_tui::settings_state::SettingsSection::Repositories);
    /// ```
    #[must_use]
    pub fn new(config: Config) -> Self {
        Self {
            config,
            section: SettingsSection::default(),
            selected_item: 0,
            edit_mode: EditMode::None,
            pending_delete: None,
        }
    }

    /// Returns a reference to the configuration.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Returns a mutable reference to the configuration.
    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
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
                    // Edit existing repository
                    let repo = &self.config.repositories[self.selected_item];
                    let path = repo.full_name();
                    let token = repo.token().unwrap_or("").to_string();
                    self.edit_mode = EditMode::EditRepository {
                        index: self.selected_item,
                        path: path.clone(),
                        path_cursor: path.len(),
                        token: token.clone(),
                        token_cursor: token.len(),
                        active_field: RepoEditField::Path,
                    };
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
                } else if self.selected_item == 1 {
                    // Auto-adjust is a toggle - toggle it directly
                    self.config.polling.auto_adjust = !self.config.polling.auto_adjust;
                }
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
                        }
                    }
                    SettingsSection::Authentication => {
                        // Set global token
                        self.config.github_token = if value.is_empty() {
                            None
                        } else {
                            Some(value.clone())
                        };
                    }
                    _ => {}
                }
                self.edit_mode = EditMode::None;
            }
            EditMode::AddRepository { value, .. } => {
                // Try to parse and add the repository
                if let Ok(repo) = Repository::parse_short(value) {
                    self.config.repositories.push(repo);
                }
                self.edit_mode = EditMode::None;
            }
            EditMode::EditRepository {
                index, path, token, ..
            } => {
                // Try to parse the path and update the repository
                if let Ok(mut repo) = Repository::parse_short(path) {
                    // Set token if provided
                    if !token.is_empty() {
                        repo = Repository::with_token(repo.owner(), repo.repo(), token);
                    }
                    if *index < self.config.repositories.len() {
                        self.config.repositories[*index] = repo;
                    }
                }
                self.edit_mode = EditMode::None;
            }
        }
    }

    /// Cancels the current edit.
    pub fn cancel_edit(&mut self) {
        self.edit_mode = EditMode::None;
    }

    /// Requests deletion of the currently selected item.
    ///
    /// This sets the item as pending deletion and requires confirmation
    /// via `confirm_delete()`. Returns `true` if an item can be deleted.
    #[must_use]
    pub fn request_delete(&mut self) -> bool {
        match self.section {
            SettingsSection::Repositories => {
                if self.selected_item < self.config.repositories.len() {
                    self.pending_delete = Some(self.selected_item);
                    true
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    /// Returns the index of the item pending deletion, if any.
    #[must_use]
    pub fn pending_delete(&self) -> Option<usize> {
        self.pending_delete
    }

    /// Confirms and executes the pending deletion.
    ///
    /// Returns `true` if an item was deleted.
    #[must_use]
    pub fn confirm_delete(&mut self) -> bool {
        if let Some(index) = self.pending_delete.take()
            && index < self.config.repositories.len()
        {
            self.config.repositories.remove(index);
            // Adjust selection if needed
            if self.selected_item >= self.config.repositories.len() && self.selected_item > 0 {
                self.selected_item -= 1;
            }
            return true;
        }
        false
    }

    /// Cancels the pending deletion.
    pub fn cancel_delete(&mut self) {
        self.pending_delete = None;
    }

    /// Returns `true` if a deletion is pending confirmation.
    #[must_use]
    pub fn is_delete_pending(&self) -> bool {
        self.pending_delete.is_some()
    }

    /// Returns `true` if the currently selected item can be deleted.
    ///
    /// Deletable items are:
    /// - Repositories (but not the "Add repository..." item at the end)
    /// - The global token in the Authentication section (when set)
    #[must_use]
    pub fn can_delete_selected(&self) -> bool {
        match self.section {
            SettingsSection::Repositories => {
                // Can delete a repository, but not the "Add repository..." item
                self.selected_item < self.config.repositories.len()
            }
            SettingsSection::Authentication => {
                // Can delete the token if it's set
                self.config
                    .github_token
                    .as_ref()
                    .is_some_and(|t| !t.is_empty())
            }
            SettingsSection::Polling => {
                // Polling settings cannot be deleted
                false
            }
        }
    }

    /// Toggles boolean settings (like auto-adjust).
    pub fn toggle_selected(&mut self) {
        match self.section {
            SettingsSection::Polling if self.selected_item == 1 => {
                self.config.polling.auto_adjust = !self.config.polling.auto_adjust;
            }
            _ => {}
        }
    }

    /// Handles character input while in edit mode.
    pub fn input_char(&mut self, ch: char) {
        self.edit_mode.insert_char(ch);
    }

    /// Switches to the next field in repository edit mode.
    ///
    /// Does nothing if not in repository edit mode.
    pub fn switch_edit_field(&mut self) {
        self.edit_mode.switch_field();
    }

    /// Handles backspace while in edit mode.
    pub fn backspace(&mut self) {
        self.edit_mode.backspace();
    }

    /// Moves the cursor left while in edit mode.
    pub fn move_cursor_left(&mut self) {
        self.edit_mode.move_cursor_left();
    }

    /// Moves the cursor right while in edit mode.
    pub fn move_cursor_right(&mut self) {
        self.edit_mode.move_cursor_right();
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
        assert_eq!(state.config().repositories.len(), 1);
        assert_eq!(state.config().repositories[0].full_name(), "owner/repo");
    }

    #[test]
    fn settings_state_delete_repository() {
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner", "repo"));

        let mut state = SettingsState::new(config);

        // Request delete - should set pending
        assert!(state.request_delete());
        assert!(state.is_delete_pending());
        assert_eq!(state.pending_delete(), Some(0));
        // Item not yet deleted
        assert_eq!(state.config().repositories.len(), 1);

        // Confirm delete
        assert!(state.confirm_delete());
        assert!(!state.is_delete_pending());
        assert!(state.config().repositories.is_empty());
    }

    #[test]
    fn settings_state_cancel_delete() {
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner", "repo"));

        let mut state = SettingsState::new(config);

        // Request delete
        assert!(state.request_delete());
        assert!(state.is_delete_pending());

        // Cancel delete
        state.cancel_delete();
        assert!(!state.is_delete_pending());
        // Item still there
        assert_eq!(state.config().repositories.len(), 1);
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
    }

    #[test]
    fn settings_state_cancel_edit() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        state.start_edit();
        state.input_char('x');
        state.cancel_edit();

        assert!(!state.is_editing());
    }

    #[test]
    fn can_delete_selected_repositories() {
        let mut config = Config::default();
        config.repositories.push(Repository::new("owner", "repo"));

        let mut state = SettingsState::new(config);

        // First item is a repository - can delete
        assert!(state.can_delete_selected());

        // Navigate to "Add repository..." item - cannot delete
        state.navigate(1);
        assert!(!state.can_delete_selected());
    }

    #[test]
    fn can_delete_selected_polling() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        state.next_section(); // Go to Polling

        // Polling interval - cannot delete
        assert!(!state.can_delete_selected());

        // Auto-adjust toggle - cannot delete
        state.navigate(1);
        assert!(!state.can_delete_selected());
    }

    #[test]
    fn can_delete_selected_authentication() {
        let mut config = Config::default();
        config.github_token = Some("test-token".to_string());

        let mut state = SettingsState::new(config);

        state.next_section(); // Polling
        state.next_section(); // Authentication

        // Token is set - can delete
        assert!(state.can_delete_selected());

        // Clear the token
        state.config_mut().github_token = None;
        assert!(!state.can_delete_selected());

        // Empty token - cannot delete
        state.config_mut().github_token = Some(String::new());
        assert!(!state.can_delete_selected());
    }

    #[test]
    fn edit_mode_cursor_movement_ascii() {
        let mut edit = EditMode::Text {
            value: "hello".to_string(),
            cursor: 5, // End of string
        };

        // Move left
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(4));

        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(3));

        // Move right
        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(4));

        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(5)); // End

        // Moving right at end does nothing
        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(5));

        // Move all the way left
        for _ in 0..10 {
            edit.move_cursor_left();
        }
        assert_eq!(edit.cursor(), Some(0));

        // Moving left at start does nothing
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(0));
    }

    #[test]
    fn edit_mode_cursor_movement_utf8() {
        // Test with multi-byte UTF-8 characters
        // "cafe" with an accent on the 'e': cafe with combining accent = 5 chars but 6 bytes
        // Or simpler: use a 2-byte character like e-acute
        let mut edit = EditMode::Text {
            value: "cafe\u{0301}".to_string(), // "cafe" + combining acute accent (cafe with accent)
            cursor: 6,                         // End of string (6 bytes)
        };

        // Move left from end - should land before the combining accent
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(4)); // The combining accent is at byte 4-5

        // Move left again - lands before 'e'
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(3));

        // Move right - lands before the combining accent
        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(4));

        // Move right - lands at end
        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(6));
    }

    #[test]
    fn edit_mode_cursor_movement_add_repository() {
        let mut edit = EditMode::AddRepository {
            value: "owner/repo".to_string(),
            cursor: 10,
        };

        // Move left
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(9));

        // Move right back
        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(10));
    }

    #[test]
    fn edit_mode_cursor_movement_edit_repository() {
        let mut edit = EditMode::EditRepository {
            index: 0,
            path: "owner/repo".to_string(),
            path_cursor: 10,
            token: "secret".to_string(),
            token_cursor: 6,
            active_field: RepoEditField::Path,
        };

        // Move cursor left in path field
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(9)); // path_cursor moved

        // Switch to token field
        edit.switch_field();

        // Move cursor left in token field
        edit.move_cursor_left();
        assert_eq!(edit.cursor(), Some(5)); // token_cursor moved

        // Move right in token field
        edit.move_cursor_right();
        assert_eq!(edit.cursor(), Some(6));
    }

    #[test]
    fn edit_mode_insert_at_cursor_position() {
        let mut edit = EditMode::Text {
            value: "hllo".to_string(),
            cursor: 1, // After 'h'
        };

        // Insert 'e' at cursor
        edit.insert_char('e');
        assert_eq!(edit.value(), Some("hello"));
        assert_eq!(edit.cursor(), Some(2)); // Cursor moved past inserted char

        // Move cursor to position 1 (after 'h'), then insert 'X'
        edit.move_cursor_left(); // Now at position 1
        edit.insert_char('X');
        assert_eq!(edit.value(), Some("hXello"));
        assert_eq!(edit.cursor(), Some(2)); // After 'X'
    }

    #[test]
    fn settings_state_cursor_methods() {
        let config = Config::default();
        let mut state = SettingsState::new(config);

        // Start editing (add repository)
        state.start_edit();

        // Type something
        state.input_char('t');
        state.input_char('e');
        state.input_char('s');
        state.input_char('t');

        // Move cursor left
        state.move_cursor_left();
        assert_eq!(state.edit_mode().cursor(), Some(3));

        // Move cursor right
        state.move_cursor_right();
        assert_eq!(state.edit_mode().cursor(), Some(4));
    }
}
