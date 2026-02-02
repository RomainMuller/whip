//! Label definitions for GitHub-based lane assignment.
//!
//! This module defines the label structure used to map GitHub issue labels
//! to Kanban board lanes. Labels follow the `whip/*` naming convention to
//! avoid conflicts with repository-specific labels.
//!
//! # Overview
//!
//! The whip application uses GitHub labels to determine which lane a task
//! belongs to. The standard labels are:
//!
//! - `whip/backlog` - Tasks waiting to be started
//! - `whip/in-progress` - Tasks currently being worked on
//! - `whip/under-review` - Tasks awaiting review or approval
//! - `whip/done` - Completed tasks (success)
//! - `whip/failed` - Tasks that failed
//!
//! # Example
//!
//! ```
//! use whip_protocol::{LabelDefinition, LaneKind, standard_status_labels};
//!
//! // Get all standard labels (including whip/failed)
//! let labels = standard_status_labels();
//! assert_eq!(labels.len(), 5);
//!
//! // Find the label for a specific lane
//! let in_progress = labels.iter()
//!     .find(|l| l.lane == LaneKind::InProgress)
//!     .unwrap();
//! assert_eq!(in_progress.name, "whip/in-progress");
//! ```

use serde::{Deserialize, Serialize};

use crate::board::LaneKind;
use crate::task::TaskState;

/// The prefix used for all whip-managed labels.
pub const LABEL_PREFIX: &str = "whip/";

/// The result of parsing a whip/* status label.
///
/// Contains both the lane assignment and the task state derived from the label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusFromLabel {
    /// The Kanban lane this label maps to.
    pub lane: LaneKind,
    /// The task state this label implies.
    pub state: TaskState,
}

/// A label definition that maps a GitHub label to a Kanban lane.
///
/// This struct defines both the label metadata (name, color, description)
/// for GitHub and the corresponding lane in the whip application.
///
/// # Examples
///
/// ```
/// use whip_protocol::{LabelDefinition, LaneKind};
///
/// let label = LabelDefinition {
///     name: "whip/backlog".to_string(),
///     color: "0052CC".to_string(),
///     description: "Task is in the backlog".to_string(),
///     lane: LaneKind::Backlog,
/// };
///
/// assert!(label.is_whip_label());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LabelDefinition {
    /// The full label name (e.g., "whip/in-progress").
    pub name: String,

    /// The hex color code without the leading `#` (e.g., "0052CC").
    pub color: String,

    /// A brief description of the label's purpose.
    pub description: String,

    /// The Kanban lane this label maps to.
    pub lane: LaneKind,
}

impl LabelDefinition {
    /// Creates a new label definition.
    ///
    /// # Arguments
    ///
    /// * `name` - The full label name
    /// * `color` - Hex color code (without `#`)
    /// * `description` - Brief description
    /// * `lane` - The corresponding Kanban lane
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::{LabelDefinition, LaneKind};
    ///
    /// let label = LabelDefinition::new(
    ///     "whip/custom",
    ///     "FF0000",
    ///     "A custom label",
    ///     LaneKind::Backlog,
    /// );
    /// ```
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        color: impl Into<String>,
        description: impl Into<String>,
        lane: LaneKind,
    ) -> Self {
        Self {
            name: name.into(),
            color: color.into(),
            description: description.into(),
            lane,
        }
    }

    /// Returns `true` if this label uses the whip prefix.
    ///
    /// # Examples
    ///
    /// ```
    /// use whip_protocol::{LabelDefinition, LaneKind};
    ///
    /// let whip_label = LabelDefinition::new("whip/backlog", "0052CC", "Backlog", LaneKind::Backlog);
    /// assert!(whip_label.is_whip_label());
    ///
    /// let other_label = LabelDefinition::new("bug", "FF0000", "Bug report", LaneKind::Backlog);
    /// assert!(!other_label.is_whip_label());
    /// ```
    #[must_use]
    pub fn is_whip_label(&self) -> bool {
        self.name.starts_with(LABEL_PREFIX)
    }
}

/// Returns the standard status labels used by whip.
///
/// These labels map directly to the Kanban lanes and use a semantic
/// color scheme that provides intuitive visual feedback:
///
/// | Label | Color | Meaning |
/// |-------|-------|---------|
/// | `whip/backlog` | Gray (#6B7280) | Neutral, waiting |
/// | `whip/in-progress` | Blue (#2563EB) | Active work |
/// | `whip/under-review` | Amber (#D97706) | Needs attention |
/// | `whip/done` | Green (#16A34A) | Success |
/// | `whip/failed` | Red (#DC2626) | Error/failure |
///
/// # Examples
///
/// ```
/// use whip_protocol::standard_status_labels;
///
/// let labels = standard_status_labels();
/// assert_eq!(labels.len(), 5);
///
/// // Labels are in lane order (with failed after done)
/// assert!(labels[0].name.ends_with("backlog"));
/// assert!(labels[1].name.ends_with("in-progress"));
/// assert!(labels[2].name.ends_with("under-review"));
/// assert!(labels[3].name.ends_with("done"));
/// assert!(labels[4].name.ends_with("failed"));
/// ```
#[must_use]
pub fn standard_status_labels() -> Vec<LabelDefinition> {
    vec![
        LabelDefinition::new(
            "whip/backlog",
            "6B7280", // Gray - neutral, waiting
            "Task is in the backlog, waiting to be started",
            LaneKind::Backlog,
        ),
        LabelDefinition::new(
            "whip/in-progress",
            "2563EB", // Blue - active work
            "Task is currently being worked on",
            LaneKind::InProgress,
        ),
        LabelDefinition::new(
            "whip/under-review",
            "D97706", // Amber - needs attention
            "Task is awaiting review or approval",
            LaneKind::UnderReview,
        ),
        LabelDefinition::new(
            "whip/done",
            "16A34A", // Green - success
            "Task has been completed successfully",
            LaneKind::Done,
        ),
        LabelDefinition::new(
            "whip/failed",
            "DC2626", // Red - error/failure
            "Task failed and needs attention",
            LaneKind::Done,
        ),
    ]
}

/// Finds the lane kind for a given label name.
///
/// Returns `Some(LaneKind)` if the label matches a standard whip label,
/// `None` otherwise.
///
/// # Examples
///
/// ```
/// use whip_protocol::{label_to_lane, LaneKind};
///
/// assert_eq!(label_to_lane("whip/backlog"), Some(LaneKind::Backlog));
/// assert_eq!(label_to_lane("whip/in-progress"), Some(LaneKind::InProgress));
/// assert_eq!(label_to_lane("whip/failed"), Some(LaneKind::Done));
/// assert_eq!(label_to_lane("bug"), None);
/// ```
#[must_use]
pub fn label_to_lane(label_name: &str) -> Option<LaneKind> {
    label_to_status(label_name).map(|s| s.lane)
}

/// Parses a label name into its lane and state.
///
/// Returns `Some(StatusFromLabel)` if the label matches a standard whip label,
/// `None` otherwise.
///
/// # Examples
///
/// ```
/// use whip_protocol::{label_to_status, LaneKind, TaskState};
///
/// let status = label_to_status("whip/done").unwrap();
/// assert_eq!(status.lane, LaneKind::Done);
/// assert_eq!(status.state, TaskState::Success);
///
/// let failed = label_to_status("whip/failed").unwrap();
/// assert_eq!(failed.lane, LaneKind::Done);
/// assert_eq!(failed.state, TaskState::Failed);
/// ```
#[must_use]
pub fn label_to_status(label_name: &str) -> Option<StatusFromLabel> {
    match label_name {
        "whip/backlog" => Some(StatusFromLabel {
            lane: LaneKind::Backlog,
            state: TaskState::Idle,
        }),
        "whip/in-progress" => Some(StatusFromLabel {
            lane: LaneKind::InProgress,
            state: TaskState::InFlight,
        }),
        "whip/under-review" => Some(StatusFromLabel {
            lane: LaneKind::UnderReview,
            state: TaskState::Idle,
        }),
        "whip/done" => Some(StatusFromLabel {
            lane: LaneKind::Done,
            state: TaskState::Success,
        }),
        "whip/failed" => Some(StatusFromLabel {
            lane: LaneKind::Done,
            state: TaskState::Failed,
        }),
        _ => None,
    }
}

/// Determines the lane from a list of labels.
///
/// Scans the provided labels for whip status labels and returns the
/// corresponding lane. If multiple whip labels are present, the first
/// one found (in standard order) takes precedence.
///
/// Returns `None` if no whip status label is found.
///
/// # Examples
///
/// ```
/// use whip_protocol::{determine_lane_from_labels, LaneKind};
///
/// let labels = vec!["bug".to_string(), "whip/in-progress".to_string()];
/// assert_eq!(determine_lane_from_labels(&labels), Some(LaneKind::InProgress));
///
/// let no_whip_labels = vec!["bug".to_string(), "enhancement".to_string()];
/// assert_eq!(determine_lane_from_labels(&no_whip_labels), None);
/// ```
#[must_use]
pub fn determine_lane_from_labels(labels: &[String]) -> Option<LaneKind> {
    determine_status_from_labels(labels).map(|s| s.lane)
}

/// Determines both lane and state from a list of labels.
///
/// Scans the provided labels for whip status labels and returns the
/// corresponding lane and task state. If multiple whip labels are present,
/// the first one found (in standard order) takes precedence.
///
/// Returns `None` if no whip status label is found.
///
/// # Examples
///
/// ```
/// use whip_protocol::{determine_status_from_labels, LaneKind, TaskState};
///
/// let labels = vec!["bug".to_string(), "whip/done".to_string()];
/// let status = determine_status_from_labels(&labels).unwrap();
/// assert_eq!(status.lane, LaneKind::Done);
/// assert_eq!(status.state, TaskState::Success);
///
/// let failed_labels = vec!["whip/failed".to_string()];
/// let failed_status = determine_status_from_labels(&failed_labels).unwrap();
/// assert_eq!(failed_status.lane, LaneKind::Done);
/// assert_eq!(failed_status.state, TaskState::Failed);
/// ```
#[must_use]
pub fn determine_status_from_labels(labels: &[String]) -> Option<StatusFromLabel> {
    // Standard whip labels in priority order
    const WHIP_LABELS: &[&str] = &[
        "whip/backlog",
        "whip/in-progress",
        "whip/under-review",
        "whip/done",
        "whip/failed",
    ];

    for &whip_label in WHIP_LABELS {
        if labels.iter().any(|l| l == whip_label) {
            return label_to_status(whip_label);
        }
    }
    None
}

/// Checks if any of the provided labels is a whip status label.
///
/// # Examples
///
/// ```
/// use whip_protocol::has_whip_status_label;
///
/// let with_whip = vec!["bug".to_string(), "whip/backlog".to_string()];
/// assert!(has_whip_status_label(&with_whip));
///
/// let without_whip = vec!["bug".to_string(), "enhancement".to_string()];
/// assert!(!has_whip_status_label(&without_whip));
/// ```
#[must_use]
pub fn has_whip_status_label(labels: &[String]) -> bool {
    determine_lane_from_labels(labels).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_definition_new() {
        let label = LabelDefinition::new("whip/test", "FF0000", "Test label", LaneKind::Backlog);

        assert_eq!(label.name, "whip/test");
        assert_eq!(label.color, "FF0000");
        assert_eq!(label.description, "Test label");
        assert_eq!(label.lane, LaneKind::Backlog);
    }

    #[test]
    fn label_definition_is_whip_label() {
        let whip_label = LabelDefinition::new("whip/test", "FF0000", "Test", LaneKind::Backlog);
        assert!(whip_label.is_whip_label());

        let other_label = LabelDefinition::new("bug", "FF0000", "Bug", LaneKind::Backlog);
        assert!(!other_label.is_whip_label());
    }

    #[test]
    fn standard_labels_count() {
        let labels = standard_status_labels();
        assert_eq!(labels.len(), 5);
    }

    #[test]
    fn standard_labels_cover_all_lanes() {
        let labels = standard_status_labels();
        let lanes: Vec<LaneKind> = labels.iter().map(|l| l.lane).collect();

        assert!(lanes.contains(&LaneKind::Backlog));
        assert!(lanes.contains(&LaneKind::InProgress));
        assert!(lanes.contains(&LaneKind::UnderReview));
        assert!(lanes.contains(&LaneKind::Done));
    }

    #[test]
    fn standard_labels_have_valid_colors() {
        let labels = standard_status_labels();

        for label in &labels {
            // Colors should be 6 hex characters
            assert_eq!(label.color.len(), 6);
            assert!(label.color.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }

    #[test]
    fn standard_labels_are_whip_labels() {
        let labels = standard_status_labels();

        for label in &labels {
            assert!(label.is_whip_label());
        }
    }

    #[test]
    fn label_to_lane_standard_labels() {
        assert_eq!(label_to_lane("whip/backlog"), Some(LaneKind::Backlog));
        assert_eq!(
            label_to_lane("whip/in-progress"),
            Some(LaneKind::InProgress)
        );
        assert_eq!(
            label_to_lane("whip/under-review"),
            Some(LaneKind::UnderReview)
        );
        assert_eq!(label_to_lane("whip/done"), Some(LaneKind::Done));
        assert_eq!(label_to_lane("whip/failed"), Some(LaneKind::Done));
    }

    #[test]
    fn label_to_status_returns_correct_states() {
        use crate::task::TaskState;

        let backlog = label_to_status("whip/backlog").unwrap();
        assert_eq!(backlog.lane, LaneKind::Backlog);
        assert_eq!(backlog.state, TaskState::Idle);

        let in_progress = label_to_status("whip/in-progress").unwrap();
        assert_eq!(in_progress.lane, LaneKind::InProgress);
        assert_eq!(in_progress.state, TaskState::InFlight);

        let review = label_to_status("whip/under-review").unwrap();
        assert_eq!(review.lane, LaneKind::UnderReview);
        assert_eq!(review.state, TaskState::Idle);

        let done = label_to_status("whip/done").unwrap();
        assert_eq!(done.lane, LaneKind::Done);
        assert_eq!(done.state, TaskState::Success);

        let failed = label_to_status("whip/failed").unwrap();
        assert_eq!(failed.lane, LaneKind::Done);
        assert_eq!(failed.state, TaskState::Failed);
    }

    #[test]
    fn label_to_lane_non_whip_labels() {
        assert_eq!(label_to_lane("bug"), None);
        assert_eq!(label_to_lane("enhancement"), None);
        assert_eq!(label_to_lane("whip/custom"), None);
    }

    #[test]
    fn label_to_status_non_whip_labels() {
        assert!(label_to_status("bug").is_none());
        assert!(label_to_status("enhancement").is_none());
        assert!(label_to_status("whip/custom").is_none());
    }

    #[test]
    fn determine_lane_from_labels_with_whip_label() {
        let labels = vec!["bug".to_string(), "whip/in-progress".to_string()];
        assert_eq!(
            determine_lane_from_labels(&labels),
            Some(LaneKind::InProgress)
        );
    }

    #[test]
    fn determine_lane_from_labels_without_whip_label() {
        let labels = vec!["bug".to_string(), "enhancement".to_string()];
        assert_eq!(determine_lane_from_labels(&labels), None);
    }

    #[test]
    fn determine_lane_from_labels_empty() {
        let labels: Vec<String> = vec![];
        assert_eq!(determine_lane_from_labels(&labels), None);
    }

    #[test]
    fn determine_lane_from_labels_multiple_whip_labels() {
        // When multiple whip labels are present, the first in lane order wins
        let labels = vec![
            "whip/done".to_string(),
            "whip/backlog".to_string(),
            "whip/in-progress".to_string(),
        ];
        // Backlog comes first in lane order
        assert_eq!(determine_lane_from_labels(&labels), Some(LaneKind::Backlog));
    }

    #[test]
    fn has_whip_status_label_true() {
        let labels = vec!["bug".to_string(), "whip/backlog".to_string()];
        assert!(has_whip_status_label(&labels));
    }

    #[test]
    fn has_whip_status_label_false() {
        let labels = vec!["bug".to_string(), "enhancement".to_string()];
        assert!(!has_whip_status_label(&labels));
    }

    #[test]
    fn label_definition_serialization_roundtrip() {
        let label = LabelDefinition::new("whip/test", "FF0000", "Test label", LaneKind::InProgress);

        let json = serde_json::to_string(&label).expect("serialize");
        let parsed: LabelDefinition = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(label, parsed);
    }
}
