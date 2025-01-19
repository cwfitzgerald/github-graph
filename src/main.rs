use eframe::egui::ahash::HashMap;
use serde::{Deserialize, Serialize};

mod display;
mod fetch;

#[derive(Deserialize, Serialize)]
struct SerializedIssueDatabase {
    issues: Vec<(IssueKey, Issue)>,
}

impl From<SerializedIssueDatabase> for IssueDatabase {
    fn from(serialized: SerializedIssueDatabase) -> Self {
        IssueDatabase {
            issues: serialized.issues.into_iter().collect(),
        }
    }
}

impl Into<SerializedIssueDatabase> for IssueDatabase {
    fn into(self) -> SerializedIssueDatabase {
        let mut issues = self.issues.into_iter().collect::<Vec<_>>();
        issues.sort_by_key(|(key, _)| key.clone());
        SerializedIssueDatabase { issues }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
#[serde(from = "SerializedIssueDatabase", into = "SerializedIssueDatabase")]
struct IssueDatabase {
    issues: HashMap<IssueKey, Issue>,
}

#[derive(Debug, Clone, Copy)]
enum StateChangeKind {
    Open,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize, PartialOrd, Ord)]
struct IssueKey {
    repo: String,
    number: u64,
}

#[derive(Deserialize, Serialize, Clone)]
struct Issue {
    repo: String,
    author: String,
    number: u64,
    title: String,
    created_at: chrono::DateTime<chrono::Utc>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    closed_reason: Option<octocrab::models::issues::IssueStateReason>,
    pr: Option<PullRequest>,
}

#[derive(Deserialize, Serialize, Clone)]
struct PullRequest {
    draft: bool,
    merged: bool,
}

impl Issue {
    fn has_event_timestamp(&self, kind: StateChangeKind) -> bool {
        match kind {
            StateChangeKind::Open => true,
            StateChangeKind::Close => self.closed_at.is_some(),
        }
    }

    fn event_timestamp(&self, kind: StateChangeKind) -> chrono::DateTime<chrono::Utc> {
        match kind {
            StateChangeKind::Open => self.created_at,
            StateChangeKind::Close => self.closed_at.unwrap(),
        }
    }

    fn key(&self) -> IssueKey {
        IssueKey {
            repo: self.repo.clone(),
            number: self.number,
        }
    }

    fn is_issue(&self) -> bool {
        self.pr.is_none()
    }

    fn is_non_draft_pr(&self) -> bool {
        self.pr.as_ref().map_or(false, |pr| !pr.draft)
    }
}

async fn main_inner() -> anyhow::Result<()> {
    let data_exists = std::fs::metadata("data.json").is_ok();

    if !data_exists {
        println!("Fetching data from Github...");
        fetch::fetch().await?;
    }

    display::display()?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), ()> {
    let result = main_inner().await;

    if let Err(err) = result {
        eprintln!("Error: {:?}", err);
        return Err(());
    }

    Ok(())
}
