use serde::{Deserialize, Serialize};

mod display;
mod fetch;

type DataType = Vec<Issue>;

#[derive(Debug, Clone, Copy)]
enum StateChangeKind {
    Open,
    Close,
}

#[derive(Deserialize, Serialize)]
struct Issue {
    repo: String,
    author: String,
    number: u64,
    title: String,
    created_at: chrono::DateTime<chrono::Utc>,
    closed_at: Option<chrono::DateTime<chrono::Utc>>,
    closed_reason: Option<octocrab::models::issues::IssueStateReason>,
    is_pr: bool,
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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let data_exists = std::fs::metadata("data.json").is_ok();

    if !data_exists {
        println!("Fetching data from Github...");
        fetch::fetch().await?;
    }

    display::display()?;

    Ok(())
}
