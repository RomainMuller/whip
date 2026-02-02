//! GitHub issue fetching and conversion to whip tasks.
//!
//! This module provides functionality for fetching GitHub issues and converting
//! them to whip [`Task`] instances for display on the Kanban board.
//!
//! # Overview
//!
//! - [`FetchOptions`]: Configuration for filtering and pagination when fetching issues
//! - [`IssueState`]: Filter for issue state (open, closed, or all)
//! - [`issue_to_task`]: Converts a GitHub issue to a whip task with deterministic IDs
//!
//! # Example
//!
//! ```no_run
//! use whip_github::{GitHubClient, FetchOptions, IssueState, issue_to_task};
//!
//! # async fn example() -> whip_github::Result<()> {
//! let client = GitHubClient::new(None).await?;
//!
//! let options = FetchOptions {
//!     state: IssueState::Open,
//!     labels: vec!["bug".to_string()],
//!     per_page: 10,
//! };
//!
//! let issues = client.fetch_issues("rust-lang", "rust", &options).await?;
//!
//! for issue in &issues {
//!     if let Some(task) = issue_to_task(issue, "rust-lang", "rust") {
//!         println!("Task: {} - {}", task.id, task.title);
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use uuid::Uuid;
use whip_protocol::{GitHubSource, Task, TaskId, determine_status_from_labels};

/// UUID namespace for generating deterministic task IDs from GitHub issues.
///
/// This is a v4 UUID chosen randomly to serve as the namespace for v5 UUID generation.
/// Using the same namespace ensures that the same issue always generates the same task ID.
const GITHUB_ISSUE_NAMESPACE: Uuid = Uuid::from_u128(0x6ba7b8109dad11d180b400c04fd430c8);

/// Options for fetching GitHub issues.
///
/// Controls filtering and pagination when retrieving issues from a repository.
///
/// # Example
///
/// ```
/// use whip_github::{FetchOptions, IssueState};
///
/// let options = FetchOptions {
///     state: IssueState::Open,
///     labels: vec!["enhancement".to_string(), "help wanted".to_string()],
///     per_page: 50,
/// };
/// ```
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    /// Filter by issue state (default: open).
    pub state: IssueState,
    /// Filter by labels (issues must have ALL these labels).
    pub labels: Vec<String>,
    /// Maximum issues to fetch per repository (default: 30, max: 100).
    pub per_page: u8,
}

impl FetchOptions {
    /// Returns the effective per_page value, clamped between 1 and 100.
    ///
    /// If `per_page` is 0, returns the default of 30.
    #[must_use]
    pub fn effective_per_page(&self) -> u8 {
        match self.per_page {
            0 => 30,
            n => n.min(100),
        }
    }
}

/// Issue state filter for GitHub API queries.
///
/// Controls which issues are returned based on their open/closed state.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IssueState {
    /// Only open issues (default).
    #[default]
    Open,
    /// Only closed issues.
    Closed,
    /// Both open and closed issues.
    All,
}

impl IssueState {
    /// Converts to the octocrab issue state parameter.
    #[must_use]
    pub fn to_octocrab_state(self) -> octocrab::params::State {
        match self {
            Self::Open => octocrab::params::State::Open,
            Self::Closed => octocrab::params::State::Closed,
            Self::All => octocrab::params::State::All,
        }
    }
}

/// Converts a GitHub issue to a whip [`Task`], if it has a whip status label.
///
/// This function creates a task with a deterministic ID based on the repository
/// and issue number, ensuring that the same issue always maps to the same task ID
/// across sessions.
///
/// Returns `None` if the issue does not have any `whip/*` status label, as such
/// issues are not managed by whip.
///
/// # ID Generation
///
/// Generates a deterministic UUID v5 from a fixed namespace and the string
/// `"{owner}/{repo}#{number}"`. This ensures:
///
/// - Same issue always produces the same task ID
/// - Different issues produce different task IDs (with astronomically high probability)
/// - IDs are stable across application restarts
///
/// # Lane Assignment
///
/// The task's lane is determined by the presence of `whip/*` labels:
///
/// | Label | Lane |
/// |-------|------|
/// | `whip/backlog` | Backlog |
/// | `whip/in-progress` | In Progress |
/// | `whip/under-review` | Under Review |
/// | `whip/done` | Done |
///
/// If multiple whip labels are present, the first one in lane order takes precedence.
///
/// # Example
///
/// ```no_run
/// use whip_github::issue_to_task;
///
/// // Assuming `issue` is an octocrab issue with a whip/backlog label
/// # fn example(issue: &octocrab::models::issues::Issue) {
/// if let Some(task) = issue_to_task(issue, "owner", "repo") {
///     println!("Task: {} in lane {:?}", task.title, task.lane);
/// }
/// # }
/// ```
#[must_use]
pub fn issue_to_task(
    issue: &octocrab::models::issues::Issue,
    owner: &str,
    repo: &str,
) -> Option<Task> {
    let number = issue.number;

    // Extract label names from issue labels
    let labels: Vec<String> = issue.labels.iter().map(|l| l.name.clone()).collect();

    // Determine lane and state from whip/* labels - if no whip label, skip this issue
    let status = determine_status_from_labels(&labels)?;

    // Generate deterministic task ID from owner/repo#number
    let id_input = format!("{owner}/{repo}#{number}");
    let id = Uuid::new_v5(&GITHUB_ISSUE_NAMESPACE, id_input.as_bytes());

    // Extract author login
    let author = issue.user.login.clone();

    // Build the GitHub source metadata
    let github = GitHubSource {
        owner: owner.to_string(),
        repo: repo.to_string(),
        number,
        url: issue.html_url.to_string(),
        labels,
        author,
        comment_count: issue.comments,
    };

    // Convert octocrab DateTime to chrono DateTime
    let created_at = issue.created_at;
    let updated_at = issue.updated_at;

    Some(Task {
        id: TaskId::from(id),
        title: issue.title.clone(),
        description: issue.body.clone().unwrap_or_default(),
        state: status.state,
        lane: status.lane,
        created_at,
        updated_at,
        github: Some(github),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use whip_protocol::{LaneKind, TaskState};

    #[test]
    fn fetch_options_default() {
        let opts = FetchOptions::default();
        assert_eq!(opts.state, IssueState::Open);
        assert!(opts.labels.is_empty());
        assert_eq!(opts.per_page, 0);
    }

    #[test]
    fn fetch_options_effective_per_page_default() {
        let opts = FetchOptions::default();
        assert_eq!(opts.effective_per_page(), 30);
    }

    #[test]
    fn fetch_options_effective_per_page_custom() {
        let opts = FetchOptions {
            per_page: 50,
            ..Default::default()
        };
        assert_eq!(opts.effective_per_page(), 50);
    }

    #[test]
    fn fetch_options_effective_per_page_clamped() {
        let opts = FetchOptions {
            per_page: 200,
            ..Default::default()
        };
        assert_eq!(opts.effective_per_page(), 100);
    }

    #[test]
    fn issue_state_default_is_open() {
        assert_eq!(IssueState::default(), IssueState::Open);
    }

    #[test]
    fn issue_state_to_octocrab() {
        assert!(matches!(
            IssueState::Open.to_octocrab_state(),
            octocrab::params::State::Open
        ));
        assert!(matches!(
            IssueState::Closed.to_octocrab_state(),
            octocrab::params::State::Closed
        ));
        assert!(matches!(
            IssueState::All.to_octocrab_state(),
            octocrab::params::State::All
        ));
    }

    #[test]
    fn deterministic_id_generation() {
        // The same owner/repo/number should always produce the same ID
        let id1 = Uuid::new_v5(&GITHUB_ISSUE_NAMESPACE, b"rust-lang/rust#12345");
        let id2 = Uuid::new_v5(&GITHUB_ISSUE_NAMESPACE, b"rust-lang/rust#12345");
        assert_eq!(id1, id2);

        // Different numbers should produce different IDs
        let id3 = Uuid::new_v5(&GITHUB_ISSUE_NAMESPACE, b"rust-lang/rust#12346");
        assert_ne!(id1, id3);

        // Different repos should produce different IDs
        let id4 = Uuid::new_v5(&GITHUB_ISSUE_NAMESPACE, b"rust-lang/cargo#12345");
        assert_ne!(id1, id4);
    }

    /// Creates a minimal valid JSON for an octocrab Issue.
    ///
    /// The octocrab Issue struct has many required fields. This helper provides
    /// a template with all required fields populated, allowing tests to focus on
    /// the fields that matter for the test case.
    fn mock_issue_json(
        number: u64,
        title: &str,
        body: Option<&str>,
        user_login: &str,
        labels: &[&str],
        comment_count: u32,
    ) -> String {
        let body_json = match body {
            Some(b) => format!(r#""{b}""#),
            None => "null".to_string(),
        };

        let labels_json: Vec<String> = labels
            .iter()
            .enumerate()
            .map(|(i, name)| {
                format!(
                    r#"{{ "id": {}, "node_id": "L{}", "url": "https://api.github.com/labels/{}", "name": "{}", "color": "d73a4a", "default": false }}"#,
                    i + 1, i + 1, name, name
                )
            })
            .collect();

        format!(
            r#"{{
            "id": 1,
            "node_id": "I_test123",
            "url": "https://api.github.com/repos/testowner/testrepo/issues/{number}",
            "repository_url": "https://api.github.com/repos/testowner/testrepo",
            "labels_url": "https://api.github.com/repos/testowner/testrepo/issues/{number}/labels",
            "comments_url": "https://api.github.com/repos/testowner/testrepo/issues/{number}/comments",
            "events_url": "https://api.github.com/repos/testowner/testrepo/issues/{number}/events",
            "html_url": "https://github.com/testowner/testrepo/issues/{number}",
            "number": {number},
            "state": "open",
            "title": "{title}",
            "body": {body_json},
            "user": {{
                "login": "{user_login}",
                "id": 123,
                "node_id": "U_test123",
                "avatar_url": "https://avatars.githubusercontent.com/u/123",
                "gravatar_id": "",
                "url": "https://api.github.com/users/{user_login}",
                "html_url": "https://github.com/{user_login}",
                "followers_url": "https://api.github.com/users/{user_login}/followers",
                "following_url": "https://api.github.com/users/{user_login}/following{{/other_user}}",
                "gists_url": "https://api.github.com/users/{user_login}/gists{{/gist_id}}",
                "starred_url": "https://api.github.com/users/{user_login}/starred{{/owner}}{{/repo}}",
                "subscriptions_url": "https://api.github.com/users/{user_login}/subscriptions",
                "organizations_url": "https://api.github.com/users/{user_login}/orgs",
                "repos_url": "https://api.github.com/users/{user_login}/repos",
                "events_url": "https://api.github.com/users/{user_login}/events{{/privacy}}",
                "received_events_url": "https://api.github.com/users/{user_login}/received_events",
                "type": "User",
                "site_admin": false
            }},
            "labels": [{}],
            "assignees": [],
            "locked": false,
            "comments": {comment_count},
            "created_at": "2024-01-15T10:30:00Z",
            "updated_at": "2024-01-20T14:45:00Z"
        }}"#,
            labels_json.join(", ")
        )
    }

    #[test]
    fn issue_to_task_converts_all_fields() {
        // Include whip/in-progress label to ensure the issue is picked up
        let issue_json = mock_issue_json(
            42,
            "Test Issue Title",
            Some("This is the issue description"),
            "testuser",
            &["bug", "whip/in-progress", "enhancement"],
            5,
        );

        let issue: octocrab::models::issues::Issue =
            serde_json::from_str(&issue_json).expect("Failed to deserialize mock issue");

        let task = issue_to_task(&issue, "testowner", "testrepo")
            .expect("Issue with whip label should produce a task");

        // Verify basic task fields
        assert_eq!(task.title, "Test Issue Title");
        assert_eq!(task.description, "This is the issue description");
        assert_eq!(task.state, TaskState::InFlight); // whip/in-progress -> InFlight
        assert_eq!(task.lane, LaneKind::InProgress);

        // Verify GitHub source metadata
        let github = task.github.expect("Task should have GitHub source");
        assert_eq!(github.owner, "testowner");
        assert_eq!(github.repo, "testrepo");
        assert_eq!(github.number, 42);
        assert_eq!(
            github.url,
            "https://github.com/testowner/testrepo/issues/42"
        );
        assert_eq!(
            github.labels,
            vec!["bug", "whip/in-progress", "enhancement"]
        );
        assert_eq!(github.author, "testuser");
        assert_eq!(github.comment_count, 5);

        // Verify deterministic ID generation
        let expected_id = Uuid::new_v5(&GITHUB_ISSUE_NAMESPACE, b"testowner/testrepo#42");
        assert_eq!(task.id, expected_id);

        // Verify timestamps are preserved
        assert_eq!(task.created_at.to_rfc3339(), "2024-01-15T10:30:00+00:00");
        assert_eq!(task.updated_at.to_rfc3339(), "2024-01-20T14:45:00+00:00");
    }

    #[test]
    fn issue_to_task_returns_none_without_whip_label() {
        let issue_json = mock_issue_json(
            99,
            "Issue without whip label",
            None,
            "anotheruser",
            &["bug"],
            0,
        );

        let issue: octocrab::models::issues::Issue =
            serde_json::from_str(&issue_json).expect("Failed to deserialize mock issue");

        let task = issue_to_task(&issue, "owner", "repo");

        // Should return None because there's no whip/* label
        assert!(task.is_none());
    }

    #[test]
    fn issue_to_task_handles_missing_body() {
        let issue_json = mock_issue_json(
            99,
            "Issue without body",
            None,
            "anotheruser",
            &["whip/backlog"],
            0,
        );

        let issue: octocrab::models::issues::Issue =
            serde_json::from_str(&issue_json).expect("Failed to deserialize mock issue");

        let task = issue_to_task(&issue, "owner", "repo")
            .expect("Issue with whip label should produce a task");

        // Body should default to empty string when null
        assert_eq!(task.description, "");
        assert_eq!(task.title, "Issue without body");
        assert_eq!(task.lane, LaneKind::Backlog);

        // GitHub metadata should still be populated
        let github = task.github.expect("Task should have GitHub source");
        assert_eq!(github.labels, vec!["whip/backlog"]);
        assert_eq!(github.author, "anotheruser");
        assert_eq!(github.comment_count, 0);
    }

    #[test]
    fn issue_to_task_same_issue_same_id() {
        let issue_json = mock_issue_json(
            123,
            "Reproducible ID",
            Some("Test body"),
            "user",
            &["whip/done"],
            0,
        );

        let issue: octocrab::models::issues::Issue =
            serde_json::from_str(&issue_json).expect("Failed to deserialize mock issue");

        let task1 = issue_to_task(&issue, "myorg", "myrepo").unwrap();
        let task2 = issue_to_task(&issue, "myorg", "myrepo").unwrap();

        assert_eq!(task1.id, task2.id);
    }

    #[test]
    fn issue_to_task_different_repos_different_ids() {
        let issue_json = mock_issue_json(1, "Same number", None, "user", &["whip/under-review"], 0);

        let issue: octocrab::models::issues::Issue =
            serde_json::from_str(&issue_json).expect("Failed to deserialize mock issue");

        let task_a = issue_to_task(&issue, "org-a", "repo").unwrap();
        let task_b = issue_to_task(&issue, "org-b", "repo").unwrap();

        assert_ne!(task_a.id, task_b.id);
    }

    #[test]
    fn issue_to_task_assigns_correct_lane_from_label() {
        // Test each label -> lane and state mapping
        let test_cases = [
            ("whip/backlog", LaneKind::Backlog, TaskState::Idle),
            (
                "whip/in-progress",
                LaneKind::InProgress,
                TaskState::InFlight,
            ),
            ("whip/under-review", LaneKind::UnderReview, TaskState::Idle),
            ("whip/done", LaneKind::Done, TaskState::Success),
            ("whip/failed", LaneKind::Done, TaskState::Failed),
        ];

        for (label, expected_lane, expected_state) in test_cases {
            let issue_json = mock_issue_json(1, "Test", None, "user", &[label], 0);
            let issue: octocrab::models::issues::Issue =
                serde_json::from_str(&issue_json).expect("Failed to deserialize mock issue");

            let task = issue_to_task(&issue, "owner", "repo")
                .expect("Issue with whip label should produce a task");

            assert_eq!(
                task.lane, expected_lane,
                "Label {} should map to lane {:?}",
                label, expected_lane
            );
            assert_eq!(
                task.state, expected_state,
                "Label {} should map to state {:?}",
                label, expected_state
            );
        }
    }
}
