use std::{
    collections::{HashMap, VecDeque},
    ops::Range,
    time::Instant,
};

use chrono::{Datelike, Days, Months, NaiveTime, TimeDelta};
use eframe::{
    egui::{self, Vec2, Vec2b, ViewportBuilder},
    NativeOptions,
};
use egui_plot::{log_grid_spacer, HLine, Legend};

use crate::{DataType, StateChangeKind};

mod plot_utils;

enum StateChange {
    Open,
    Close,
}

struct Event<'a> {
    issue: &'a crate::Issue,
    state_change: StateChange,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn display() -> anyhow::Result<()> {
    let data = std::fs::read_to_string("data.json")?;
    let issues: DataType = serde_json::from_str(&data)?;

    let start_time = Instant::now();

    let issue_points = live_issues(issues.iter().filter(|issue| !issue.is_pr));
    let pr_points = live_issues(issues.iter().filter(|issue| issue.is_pr));

    let closed_issue_points = closed_issues(issues.iter().filter(|issue| !issue.is_pr));
    let closed_pr_points = closed_issues(issues.iter().filter(|issue| issue.is_pr));

    let open_issue_points_per_month = closed_issues_per_month(
        issues.iter().filter(|issue| !issue.is_pr),
        StateChangeKind::Open,
    );

    let open_pr_points_per_month = closed_issues_per_month(
        issues.iter().filter(|issue| issue.is_pr),
        StateChangeKind::Open,
    );

    let closed_issue_points_per_month = closed_issues_per_month(
        issues.iter().filter(|issue| !issue.is_pr),
        StateChangeKind::Close,
    );
    let closed_pr_points_per_month = closed_issues_per_month(
        issues.iter().filter(|issue| issue.is_pr),
        StateChangeKind::Close,
    );

    let BarsAndDiff {
        bars:
            Stacked {
                top: closed_issue_bars,
                bottom: open_issue_bars,
            },
        diff: issue_diff_points,
    } = points_to_stacked_bars(
        Stacked {
            top: &closed_issue_points_per_month,
            bottom: &open_issue_points_per_month,
        },
        24.0 * 60.0 * 60.0,
        true,
    );

    let BarsAndDiff {
        bars: Stacked {
            top: closed_pr_bars,
            bottom: open_pr_bars,
        },
        diff: pr_diff_points,
    } = points_to_stacked_bars(
        Stacked {
            top: &closed_pr_points_per_month,
            bottom: &open_pr_points_per_month,
        },
        24.0 * 60.0 * 60.0,
        true,
    );

    let months_of_issues = issues_associated_with_month(issues.iter().filter(|issue| !issue.is_pr));
    let months_of_prs = issues_associated_with_month(issues.iter().filter(|issue| issue.is_pr));

    let buckets = [
        Bucket {
            age: TimeDelta::days(1),
            name: "1 Day",
        },
        Bucket {
            age: TimeDelta::days(7),
            name: "1 Week",
        },
        Bucket {
            age: TimeDelta::days(14),
            name: "2 Weeks",
        },
        Bucket {
            age: TimeDelta::days(30),
            name: "30 Days",
        },
        Bucket {
            age: TimeDelta::days(90),
            name: "90 Days",
        },
        Bucket {
            age: TimeDelta::days(180),
            name: "180 Days",
        },
        Bucket {
            age: TimeDelta::days(365),
            name: "1 Year",
        },
    ];

    let issue_lines = points_per_month_of_issues(&months_of_issues, &buckets);
    let pr_lines = points_per_month_of_issues(&months_of_prs, &buckets);

    eprintln!("Computed in {:?}", start_time.elapsed());

    let mut current_tab = 0;

    let mut tab_1_pr_or_issue = 0;

    let mut native_options = NativeOptions::default();

    // Set the viewport to the perfect size for social previews
    native_options.viewport = ViewportBuilder::default().with_inner_size(Vec2::new(1000.0, 470.0));

    eframe::run_simple_native(
        "Github Stats Visualizer",
        native_options,
        move |ctx, _frame| {
            egui::CentralPanel::default().show(ctx, |ui| {
                let mut clicked = false;
                ui.horizontal(|ui| {
                    clicked |= ui.selectable_value(&mut current_tab, 0, "Totals").clicked();
                    clicked |= ui.selectable_value(&mut current_tab, 1, "Closed").clicked();
                });

                if current_tab == 0 {
                    egui_plot::Plot::new("Plot Totals")
                        .show_axes(true)
                        .show_grid(true)
                        .legend(Legend::default().position(egui_plot::Corner::LeftTop))
                        .x_grid_spacer(plot_utils::grid_spacer)
                        .y_grid_spacer(log_grid_spacer(10))
                        .x_axis_formatter(plot_utils::x_formatter)
                        .label_formatter(plot_utils::label_formatter)
                        .show(ui, |ui| {
                            if clicked {
                                ui.set_auto_bounds(Vec2b::TRUE);
                            }

                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    issue_points.clone(),
                                ))
                                .name("Issues"),
                            );
                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    pr_points.clone(),
                                ))
                                .name("PRs"),
                            );
                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    closed_issue_points.clone(),
                                ))
                                .name("Closed Issues"),
                            );
                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    closed_pr_points.clone(),
                                ))
                                .name("Closed PRs"),
                            );
                        });
                } else if current_tab == 1 {
                    ui.horizontal(|ui| {
                        ui.selectable_value(&mut tab_1_pr_or_issue, 0, "Issues")
                            .clicked();
                        ui.selectable_value(&mut tab_1_pr_or_issue, 1, "PRs")
                            .clicked();
                    });

                    let (
                        open_points,
                        open_bars,
                        closed_points,
                        closed_bars,
                        diff_points,
                        age_lines,
                        name,
                    ) = if tab_1_pr_or_issue == 0 {
                        (
                            &open_issue_points_per_month,
                            &open_issue_bars,
                            &closed_issue_points_per_month,
                            &closed_issue_bars,
                            &issue_diff_points,
                            &issue_lines,
                            "Issues",
                        )
                    } else {
                        (
                            &open_pr_points_per_month,
                            &open_pr_bars,
                            &closed_pr_points_per_month,
                            &closed_pr_bars,
                            &pr_diff_points,
                            &pr_lines,
                            "PRs",
                        )
                    };

                    let remaining_height = ui.available_size_before_wrap().y;

                    egui_plot::Plot::new("Open Closed Ratio")
                        .show_axes(true)
                        .show_grid(true)
                        .legend(Legend::default().position(egui_plot::Corner::LeftTop))
                        .allow_zoom(Vec2b::new(true, false))
                        .allow_drag(Vec2b::new(true, false))
                        .x_grid_spacer(plot_utils::grid_spacer)
                        .x_axis_formatter(plot_utils::x_formatter)
                        .y_grid_spacer(log_grid_spacer(10))
                        .y_axis_min_width(25.0)
                        .set_margin_fraction(Vec2::ZERO)
                        .label_formatter(plot_utils::label_formatter)
                        .height(remaining_height / 3.0)
                        .link_axis("Open/Closed", Vec2b::new(true, false))
                        .link_cursor("Open/Closed", Vec2b::new(true, false))
                        .show(ui, |ui| {
                            if clicked {
                                ui.set_auto_bounds(Vec2b::TRUE);
                            }

                            ui.bar_chart(
                                egui_plot::BarChart::new(open_bars.clone())
                                    .name(format!("Percentage Open {name}"))
                                    .color(egui::Color32::from_rgb(0x3f, 0xb9, 0x50)),
                            );
                            ui.bar_chart(
                                egui_plot::BarChart::new(closed_bars.clone())
                                    .name(format!("Percentage Closed {name}"))
                                    .color(egui::Color32::from_rgb(0xab, 0x7d, 0xf8)),
                            );

                            ui.hline(HLine::new(0.5).name("50%"));
                        });

                    egui_plot::Plot::new("Open Closed Values")
                        .show_axes(true)
                        .show_grid(true)
                        .legend(Legend::default().position(egui_plot::Corner::LeftTop))
                        .allow_zoom(Vec2b::new(true, false))
                        .allow_drag(Vec2b::new(true, false))
                        .x_grid_spacer(plot_utils::grid_spacer)
                        .x_axis_formatter(plot_utils::x_formatter)
                        .y_grid_spacer(log_grid_spacer(10))
                        .y_axis_min_width(25.0)
                        .set_margin_fraction(Vec2::ZERO)
                        .label_formatter(plot_utils::label_formatter)
                        .height(remaining_height / 3.0)
                        .link_axis("Open/Closed", Vec2b::new(true, false))
                        .link_cursor("Open/Closed", Vec2b::new(true, false))
                        .show(ui, |ui| {
                            if clicked {
                                ui.set_auto_bounds(Vec2b::TRUE);
                            }

                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    open_points.clone(),
                                ))
                                .name(format!("Opened {name}"))
                                .color(egui::Color32::from_rgb(0x3f, 0xb9, 0x50)),
                            );

                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    closed_points.clone(),
                                ))
                                .name(format!("Closed {name}"))
                                .color(egui::Color32::from_rgb(0xab, 0x7d, 0xf8)),
                            );
                        });

                    egui_plot::Plot::new("Difference Values")
                        .show_axes(true)
                        .show_grid(true)
                        .legend(Legend::default().position(egui_plot::Corner::LeftTop))
                        .allow_zoom(Vec2b::new(true, false))
                        .allow_drag(Vec2b::new(true, false))
                        .x_grid_spacer(plot_utils::grid_spacer)
                        .x_axis_formatter(plot_utils::x_formatter)
                        .y_grid_spacer(log_grid_spacer(5))
                        .y_axis_min_width(25.0)
                        .center_y_axis(true)
                        .set_margin_fraction(Vec2::ZERO)
                        .label_formatter(plot_utils::label_formatter)
                        .height(remaining_height / 3.0)
                        .link_axis("Open/Closed", Vec2b::new(true, false))
                        .link_cursor("Open/Closed", Vec2b::new(true, false))
                        .show(ui, |ui| {
                            if clicked {
                                ui.set_auto_bounds(Vec2b::TRUE);
                            }

                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    diff_points.clone(),
                                ))
                                .name(format!("{name} Count Diff"))
                                .color(egui::Color32::from_rgb(0xcc, 0x86, 0x0d)),
                            );
                        });
                } else if current_tab == 2 {
                    egui_plot::Plot::new("Issue Age")
                        .show_axes(true)
                        .show_grid(true)
                        .legend(Legend::default().position(egui_plot::Corner::LeftTop))
                        .allow_zoom(Vec2b::new(true, false))
                        .allow_drag(Vec2b::new(true, false))
                        .x_grid_spacer(plot_utils::grid_spacer)
                        .x_axis_formatter(plot_utils::x_formatter)
                        .y_grid_spacer(log_grid_spacer(10))
                        .y_axis_min_width(25.0)
                        .set_margin_fraction(Vec2::ZERO)
                        .label_formatter(plot_utils::label_formatter)
                        .show(ui, |ui| {
                            if clicked {
                                ui.set_auto_bounds(Vec2b::TRUE);
                            }

                            for line in age_lines {
                                ui.line(line.clone());
                            }
                        });
                }
            });
        },
    )
    .unwrap();

    Ok(())
}

struct BarsAndDiff {
    bars: Stacked<Vec<egui_plot::Bar>>,
    diff: Vec<egui_plot::PlotPoint>,
}

struct Stacked<T> {
    top: T,
    bottom: T,
}

fn points_to_stacked_bars(
    points: Stacked<&[egui_plot::PlotPoint]>,
    width: f64,
    normalize: bool,
) -> BarsAndDiff {
    let mut top_bars = Vec::new();
    let mut bottom_bars = Vec::new();
    let mut diff = Vec::new();

    let mut top_iter = points.top.iter();
    let mut bottom_iter = points.bottom.iter();

    let mut top_point = top_iter.next();
    let mut bottom_point = bottom_iter.next();

    let bar = |x: f64, y: f64| egui_plot::Bar::new(x, y).width(width);

    while let (Some(top), Some(bottom)) = (top_point, bottom_point) {
        if top.x == bottom.x {
            let mut top_value = top.y;
            let mut bottom_value = bottom.y;

            diff.push(egui_plot::PlotPoint::new(top.x, bottom_value - top_value));

            let total = top_value + bottom_value;

            if normalize && total != 0.0 {
                top_value /= total;
                bottom_value /= total;
            }

            top_bars.push(bar(top.x, top_value).base_offset(bottom_value));
            bottom_bars.push(bar(bottom.x, bottom_value));
            top_point = top_iter.next();
            bottom_point = bottom_iter.next();
        } else if top.x < bottom.x {
            let mut top_value = top.y;

            diff.push(egui_plot::PlotPoint::new(top.x, -top_value));

            if normalize {
                top_value = 1.0;
            }

            top_bars.push(bar(top.x, top_value));
            top_point = top_iter.next();
        } else {
            let mut bottom_value = bottom.y;

            diff.push(egui_plot::PlotPoint::new(bottom.x, bottom_value));

            if normalize {
                bottom_value = 1.0;
            }

            bottom_bars.push(bar(bottom.x, bottom_value));
            bottom_point = bottom_iter.next();
        }
    }

    while let Some(top) = top_point {
        let mut top_value = top.y;

        diff.push(egui_plot::PlotPoint::new(top.x, -top_value));

        if normalize {
            top_value = 1.0;
        }

        top_bars.push(bar(top.x, top_value));
        top_point = top_iter.next();
    }

    while let Some(bottom) = bottom_point {
        let mut bottom_value = bottom.y;

        diff.push(egui_plot::PlotPoint::new(bottom.x, bottom_value));

        if normalize {
            bottom_value = 1.0;
        }

        bottom_bars.push(bar(bottom.x, bottom_value));
        bottom_point = bottom_iter.next();
    }

    let bars = Stacked {
        top: top_bars,
        bottom: bottom_bars,
    };

    BarsAndDiff { bars, diff }
}

fn generate_events<'a, I>(issues: I) -> Vec<Event<'a>>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let mut events = Vec::new();

    for issue in issues {
        if let Some(closed_at) = issue.closed_at {
            events.push(Event {
                issue,
                state_change: StateChange::Close,
                timestamp: closed_at,
            });
        }

        events.push(Event {
            issue,
            state_change: StateChange::Open,
            timestamp: issue.created_at,
        });
    }

    events.sort_unstable_by_key(|event| event.timestamp);
    events
}

fn live_issues<'a, I>(issues: I) -> Vec<egui_plot::PlotPoint>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let events = generate_events(issues);

    let mut issue_points = Vec::new();

    let mut open_issues = 0;

    for event in events {
        issue_points.push(egui_plot::PlotPoint::new(
            event.timestamp.timestamp() as f64,
            open_issues,
        ));

        match event.state_change {
            StateChange::Open => open_issues += 1,
            StateChange::Close => open_issues -= 1,
        }

        issue_points.push(egui_plot::PlotPoint::new(
            event.timestamp.timestamp() as f64,
            open_issues,
        ));
    }
    issue_points
}

fn closed_issues<'a, I>(issues: I) -> Vec<egui_plot::PlotPoint>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let mut issues_sorted: Vec<_> = issues
        .into_iter()
        .filter(|issue| issue.closed_at.is_some())
        .collect();
    issues_sorted.sort_unstable_by_key(|issue| issue.closed_at);

    let mut issue_points = Vec::new();

    let mut closed = 0;

    for issue in issues_sorted {
        let closed_at = issue.closed_at.unwrap();

        issue_points.push(egui_plot::PlotPoint::new(
            closed_at.timestamp() as f64,
            closed,
        ));

        closed += 1;

        issue_points.push(egui_plot::PlotPoint::new(
            closed_at.timestamp() as f64,
            closed,
        ));
    }

    issue_points
}

fn closed_issues_per_month<'a, I>(issues: I, kind: StateChangeKind) -> Vec<egui_plot::PlotPoint>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let mut issues_sorted: Vec<_> = issues
        .into_iter()
        .filter(|issue| issue.has_event_timestamp(kind))
        .collect();
    issues_sorted.sort_unstable_by_key(|issue| issue.event_timestamp(kind));

    let mut issue_points = Vec::new();

    let first_triggered_date = issues_sorted
        .first()
        .unwrap()
        .event_timestamp(kind)
        .with_time(NaiveTime::MIN)
        .earliest()
        .unwrap();
    let last_triggered_date = issues_sorted
        .last()
        .unwrap()
        .event_timestamp(kind)
        .with_time(NaiveTime::MIN)
        .earliest()
        .unwrap()
        .checked_add_days(Days::new(1))
        .unwrap();

    let mut current_date = first_triggered_date;
    let mut current_issue_index = 0;

    // 30 day rolling average of triggered issues
    let mut triggered = VecDeque::new();

    while current_date < last_triggered_date {
        // Get all issues that are closed on this date
        for &issue in &issues_sorted[current_issue_index..] {
            if issue.event_timestamp(kind).date_naive() == current_date.date_naive() {
                triggered.push_back(issue);
            } else {
                break;
            }
            current_issue_index += 1;
        }

        let cutoff_date = current_date.checked_sub_days(Days::new(30)).unwrap();

        // Remove issues that are older than 30 days
        while let Some(issue) = triggered.front() {
            let issue_date = issue.event_timestamp(kind).date_naive();
            if issue_date < cutoff_date.date_naive() {
                triggered.pop_front();
            } else {
                break;
            }
        }

        let middle_date = current_date.checked_sub_days(Days::new(15)).unwrap();

        issue_points.push(egui_plot::PlotPoint::new(
            middle_date.timestamp() as f64,
            triggered.len() as f64,
        ));

        current_date = current_date.checked_add_days(Days::new(1)).unwrap();
    }

    issue_points
}

struct MonthOfIssues<'a> {
    timestamp: chrono::DateTime<chrono::Utc>,
    issues: Vec<&'a crate::Issue>,
}

fn issues_associated_with_month<'a, I>(issues: I) -> Vec<MonthOfIssues<'a>>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let events = generate_events(issues);

    let mut open_issues: HashMap<u64, Event<'a>> = HashMap::new();
    let mut months_of_issues = Vec::<MonthOfIssues<'a>>::new();

    let mut current_time = events.first().unwrap().timestamp;
    let mut current_month = current_time.month0();
    let mut current_issues = Vec::new();

    for event in events {
        let month = event.timestamp.month0();

        if month != current_month {
            // Add all open issues to the current month
            current_issues.extend(open_issues.values().map(|event| event.issue));

            months_of_issues.push(MonthOfIssues {
                timestamp: current_time,
                issues: current_issues,
            });

            current_time = current_time.checked_add_months(Months::new(1)).unwrap();
            current_month = month;
            current_issues = Vec::new();
        }

        match event.state_change {
            StateChange::Open => {
                open_issues.insert(event.issue.number, event);
            }
            StateChange::Close => {
                open_issues.remove(&event.issue.number).unwrap();
                current_issues.push(event.issue);
            }
        }
    }

    months_of_issues
}

struct Bucket {
    age: TimeDelta,
    name: &'static str,
}

fn points_per_month_of_issues(
    months: &[MonthOfIssues<'_>],
    buckets: &[Bucket],
) -> Vec<egui_plot::Line> {
    let mut lines = vec![Vec::new(); buckets.len()];

    for month in months {
        let mut counts = vec![0; buckets.len()];

        for issue in &month.issues {
            let age = month.timestamp - issue.created_at;

            for (i, bucket) in buckets.iter().enumerate().rev() {
                if age < bucket.age {
                    counts[i] += 1;
                    break;
                }
            }
        }

        for (i, count) in counts.iter().enumerate() {
            lines[i].push(egui_plot::PlotPoint::new(
                month.timestamp.timestamp() as f64,
                *count as f64,
            ));
        }
    }

    let mut result = Vec::with_capacity(buckets.len());
    for (points, bucket) in lines.into_iter().zip(buckets) {
        result.push(egui_plot::Line::new(egui_plot::PlotPoints::Owned(points)).name(&bucket.name));
    }
    result
}

#[allow(dead_code)]
fn remap(value: f64, from: Range<f64>, to: Range<f64>) -> f64 {
    to.start + (value - from.start) / (from.end - from.start) * (to.end - to.start)
}

#[allow(dead_code)]
fn gaussian(x: f64) -> f64 {
    let o = 0.4;
    let u = 0.0;

    let normalizing_constant = (o * (2.0 * std::f64::consts::PI).sqrt()).recip();

    let exponent = -((x - u).powi(2) / (2.0 * o.powi(2)));

    normalizing_constant * exponent.exp()
}
