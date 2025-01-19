use std::io::Write;

use chrono::{DateTime, Local};
use futures_util::TryStreamExt;
use octocrab::models::Rate;

use crate::IssueDatabase;

pub async fn fetch() -> anyhow::Result<()> {
    let crab = octocrab::Octocrab::builder()
        .personal_token(std::fs::read_to_string(".token").unwrap().trim())
        .build()?;

    print_rate(&crab.ratelimit().get().await?.resources.core);

    let mut issues = IssueDatabase::default();
    issues_from_repo(&crab, "gfx-rs", "wgpu", &mut issues).await?;
    issues_from_repo(&crab, "gfx-rs", "naga", &mut issues).await?;
    issues_from_repo(&crab, "gfx-rs", "wgpu-rs", &mut issues).await?;

    eprintln!("Done");

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
    issues: &mut IssueDatabase,
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

    let repo_full = format!("{}/{}", owner, repo);

    let mut pulled_values = 0_usize;

    while let Some(issue) = issue_stream.try_next().await? {
        if pulled_values % 100 == 0 {
            print!("{repo_full} Issues Collected: {}\r", pulled_values);
            std::io::stdout().lock().flush()?;
        }
        pulled_values += 1;

        issues.issues.insert(
            super::IssueKey {
                repo: repo_full.clone(),
                number: issue.number,
            },
            super::Issue {
                repo: repo_full.clone(),
                author: issue.user.login,
                number: issue.number,
                title: issue.title,
                created_at: issue.created_at,
                closed_at: issue.closed_at,
                closed_reason: issue.state_reason,
                pr: None,
            },
        );
    }

    println!("{repo_full} Issues Collected: {}", pulled_values);
    pulled_values = 0;

    let pr_query = crab
        .pulls(owner, repo)
        .list()
        .per_page(100)
        .state(octocrab::params::State::All)
        .sort(octocrab::params::pulls::Sort::Created)
        .direction(octocrab::params::Direction::Ascending)
        .send()
        .await?;

    tokio::pin! {
        let pr_stream = pr_query.into_stream(&crab);
    };

    while let Some(pr) = pr_stream.try_next().await? {
        if pulled_values % 100 == 0 {
            print!("{repo_full} PRs Collected: {}\r", pulled_values);
            std::io::stdout().lock().flush()?;
        }
        pulled_values += 1;

        issues
            .issues
            .get_mut(&super::IssueKey {
                repo: repo_full.clone(),
                number: pr.number,
            })
            .unwrap()
            .pr = Some(super::PullRequest {
            draft: pr.draft.unwrap_or(false),
            merged: pr.merged.unwrap_or(false),
        });
    }

    println!("{repo_full} PRs Collected: {}", pulled_values);

    Ok(())
}
