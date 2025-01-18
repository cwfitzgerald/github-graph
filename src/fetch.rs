use std::io::Write;

use chrono::{DateTime, Local};
use futures_util::TryStreamExt;
use octocrab::models::Rate;

pub async fn fetch() -> anyhow::Result<()> {
    let crab = octocrab::Octocrab::builder()
        .personal_token(std::fs::read_to_string(".token").unwrap().trim())
        .build()?;

    let mut issues = Vec::new();
    issues_from_repo(&crab, "gfx-rs", "wgpu", &mut issues).await?;
    issues_from_repo(&crab, "gfx-rs", "naga", &mut issues).await?;
    issues_from_repo(&crab, "gfx-rs", "wgpu-rs", &mut issues).await?;

    println!();

    std::fs::write("data.json", serde_json::to_string_pretty(&issues)?)?;

    print_rate(&crab.ratelimit().get().await?.resources.core);

    Ok(())
}

fn print_rate(rate: &Rate) {
    println!(
        "Rate Limit: {}/{} (Reset {})",
        rate.used,
        rate.limit,
        DateTime::from_timestamp(rate.reset as i64, 0)
            .unwrap()
            .with_timezone(&Local)
    );
}

async fn issues_from_repo(
    crab: &octocrab::Octocrab,
    owner: &str,
    repo: &str,
    issues: &mut Vec<super::Issue>,
) -> anyhow::Result<()> {
    let issue_query = crab
        .issues(owner, repo)
        .list()
        .per_page(100)
        .state(octocrab::params::State::All)
        .sort(octocrab::params::issues::Sort::Created)
        .direction(octocrab::params::Direction::Ascending)
        .send()
        .await?;
    tokio::pin! {
        let issue_stream = issue_query.into_stream(&crab);
    };

    while let Some(issue) = issue_stream.try_next().await? {
        if issues.len() % 100 == 0 {
            print!("Issues Collected: {}\r", issues.len());
            std::io::stdout().lock().flush()?;
        }
        issues.push(super::Issue {
            repo: format!("{}/{}", owner, repo),
            author: issue.user.login,
            number: issue.number,
            title: issue.title,
            created_at: issue.created_at,
            closed_at: issue.closed_at,
            closed_reason: issue.state_reason,
            is_pr: issue.pull_request.is_some(),
        })
    }
    Ok(())
}
