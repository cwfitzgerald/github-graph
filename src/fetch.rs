use std::io::Write;

use futures_util::TryStreamExt;

pub async fn fetch() -> anyhow::Result<()> {
    let crab = octocrab::Octocrab::builder()
        .personal_token(std::fs::read_to_string(".token").unwrap().trim())
        .build()?;

    let mut issues = Vec::new();
    issues_from_repo(&crab, "gfx-rs", "wgpu", &mut issues).await?;
    issues_from_repo(&crab, "gfx-rs", "naga", &mut issues).await?;
    issues_from_repo(&crab, "gfx-rs", "wgpu-rs", &mut issues).await?;

    std::fs::write("data.json", serde_json::to_string_pretty(&issues)?)?;

    dbg!(crab.ratelimit().get().await?);

    Ok(())
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
    println!();
    Ok(())
}
