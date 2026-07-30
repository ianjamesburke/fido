use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoteDirection {
    Up,
    Down,
}

impl VoteDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            VoteDirection::Up => "up",
            VoteDirection::Down => "down",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "up" => Some(VoteDirection::Up),
            "down" => Some(VoteDirection::Down),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ColorScheme {
    #[default]
    Default,
    Dark,
    Light,
    Solarized,
}

impl ColorScheme {
    pub fn as_str(&self) -> &'static str {
        match self {
            ColorScheme::Default => "Default",
            ColorScheme::Dark => "Dark",
            ColorScheme::Light => "Light",
            ColorScheme::Solarized => "Solarized",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Default" => Some(ColorScheme::Default),
            "Dark" => Some(ColorScheme::Dark),
            "Light" => Some(ColorScheme::Light),
            "Solarized" => Some(ColorScheme::Solarized),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SortOrder {
    #[default]
    Newest,
    Popular,
    Controversial,
}

impl SortOrder {
    pub fn as_str(&self) -> &'static str {
        match self {
            SortOrder::Newest => "Newest",
            SortOrder::Popular => "Popular",
            SortOrder::Controversial => "Controversial",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "Newest" => Some(SortOrder::Newest),
            "Popular" => Some(SortOrder::Popular),
            "Controversial" => Some(SortOrder::Controversial),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MembershipRole {
    Admin,
    Contributor,
    Member,
}

impl MembershipRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            MembershipRole::Admin => "admin",
            MembershipRole::Contributor => "contributor",
            MembershipRole::Member => "member",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(MembershipRole::Admin),
            "contributor" => Some(MembershipRole::Contributor),
            "member" => Some(MembershipRole::Member),
            _ => None,
        }
    }
}

/// How a repo surfaced in the community browser. A repo can reach the browser
/// through more than one relationship, so items carry a set of these rather
/// than a single value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoSource {
    /// The user starred the repo (`GET /user/starred`).
    Starred,
    /// The repo's GitHub owner login is the user's own login.
    Owned,
    /// The user is a collaborator or org member on the repo, GitHub's closest
    /// listable notion of "contributed to".
    Contributor,
}

impl RepoSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            RepoSource::Starred => "starred",
            RepoSource::Owned => "owned",
            RepoSource::Contributor => "contributor",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "starred" => Some(RepoSource::Starred),
            "owned" => Some(RepoSource::Owned),
            "contributor" => Some(RepoSource::Contributor),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationType {
    Mention,
    Reply,
    DmRequest,
    ThreadApproved,
    ThreadRejected,
}

impl NotificationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            NotificationType::Mention => "mention",
            NotificationType::Reply => "reply",
            NotificationType::DmRequest => "dm_request",
            NotificationType::ThreadApproved => "thread_approved",
            NotificationType::ThreadRejected => "thread_rejected",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mention" => Some(NotificationType::Mention),
            "reply" => Some(NotificationType::Reply),
            "dm_request" => Some(NotificationType::DmRequest),
            "thread_approved" => Some(NotificationType::ThreadApproved),
            "thread_rejected" => Some(NotificationType::ThreadRejected),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DmConversationState {
    Pending,
    Accepted,
    Declined,
}

impl DmConversationState {
    pub fn as_str(&self) -> &'static str {
        match self {
            DmConversationState::Pending => "pending",
            DmConversationState::Accepted => "accepted",
            DmConversationState::Declined => "declined",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(DmConversationState::Pending),
            "accepted" => Some(DmConversationState::Accepted),
            "declined" => Some(DmConversationState::Declined),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Issue,
    PullRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityState {
    Open,
    Closed,
    Merged,
}
