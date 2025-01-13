use std::{
    collections::VecDeque,
    ops::{Range, RangeInclusive},
};

use chrono::{DateTime, Datelike, Days, Month, Months, NaiveDate, NaiveTime, TimeDelta, Utc};
use eframe::{
    egui::{self, ahash::HashMap, Vec2, ViewportBuilder},
    NativeOptions,
};
use egui_plot::{log_grid_spacer, GridInput, GridMark, Legend};

use crate::DataType;

enum StateChange {
    Open,
    Close,
}

struct Event {
    state_change: StateChange,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub fn display() -> anyhow::Result<()> {
    let data = std::fs::read_to_string("data.json")?;
    let issues: DataType = serde_json::from_str(&data)?;

    let issue_points = live_issues(issues.iter().filter(|issue| !issue.is_pr));
    let pr_points = live_issues(issues.iter().filter(|issue| issue.is_pr));

    let closed_issue_points = closed_issues(issues.iter().filter(|issue| !issue.is_pr));
    let closed_pr_points = closed_issues(issues.iter().filter(|issue| issue.is_pr));

    let closed_issue_points_per_month =
        closed_issues_per_month(issues.iter().filter(|issue| !issue.is_pr));
    let closed_pr_points_per_month =
        closed_issues_per_month(issues.iter().filter(|issue| issue.is_pr));

    let mut current_tab = 0;

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

                let mut plot = egui_plot::Plot::new("Issue Count");
                if clicked {
                    plot = plot.reset()
                }

                plot.show_axes(true)
                    .show_grid(true)
                    .legend(Legend::default().position(egui_plot::Corner::LeftTop))
                    .x_grid_spacer(grid_spacer)
                    .y_grid_spacer(log_grid_spacer(10))
                    .x_axis_formatter(x_formatter)
                    .label_formatter(|name, point| {
                        let date = chrono::DateTime::from_timestamp(point.x as i64, 0)
                            .unwrap()
                            .date_naive();

                        if name == "" {
                            return format!("{date}");
                        }

                        format!("{name} on {date}: {}", point.y)
                    })
                    .show(ui, |ui| {
                        if current_tab == 0 {
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
                        }

                        if current_tab == 1 {
                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    closed_issue_points_per_month.clone(),
                                ))
                                .name("Closed Issues per Month"),
                            );

                            ui.line(
                                egui_plot::Line::new(egui_plot::PlotPoints::Owned(
                                    closed_pr_points_per_month.clone(),
                                ))
                                .name("Closed PRs per Month"),
                            );
                        }
                    })
            });
        },
    )
    .unwrap();

    Ok(())
}

fn live_issues<'a, I>(issues: I) -> Vec<egui_plot::PlotPoint>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let mut events = Vec::new();

    for issue in issues {
        if let Some(closed_at) = issue.closed_at {
            events.push(Event {
                state_change: StateChange::Close,
                timestamp: closed_at,
            });
        }

        events.push(Event {
            state_change: StateChange::Open,
            timestamp: issue.created_at,
        });
    }

    events.sort_unstable_by_key(|event| event.timestamp);

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

fn closed_issues_per_month<'a, I>(issues: I) -> Vec<egui_plot::PlotPoint>
where
    I: IntoIterator<Item = &'a crate::Issue>,
{
    let mut issues_sorted: Vec<_> = issues
        .into_iter()
        .filter(|issue| issue.closed_at.is_some())
        .collect();
    issues_sorted.sort_unstable_by_key(|issue| issue.closed_at);

    let mut issue_points = Vec::new();

    let first_closed_date = issues_sorted
        .first()
        .unwrap()
        .closed_at
        .unwrap()
        .with_time(NaiveTime::MIN)
        .earliest()
        .unwrap();
    let last_closed_date = issues_sorted
        .last()
        .unwrap()
        .closed_at
        .unwrap()
        .with_time(NaiveTime::MIN)
        .earliest()
        .unwrap()
        .checked_add_days(Days::new(1))
        .unwrap();

    let mut current_date = first_closed_date;
    let mut current_issue_index = 0;

    // 30 day rolling average of closed issues
    let mut closed = VecDeque::new();

    while current_date < last_closed_date {
        // Get all issues that are closed on this date
        for &issue in &issues_sorted[current_issue_index..] {
            if issue.closed_at.unwrap().date_naive() == current_date.date_naive() {
                closed.push_back(issue);
            } else {
                break;
            }
            current_issue_index += 1;
        }

        let cutoff_date = current_date.checked_sub_days(Days::new(30)).unwrap();

        // Remove issues that are older than 30 days
        while let Some(issue) = closed.front() {
            let issue_date = issue.closed_at.unwrap().date_naive();
            if issue_date < cutoff_date.date_naive() {
                closed.pop_front();
            } else {
                break;
            }
        }

        let middle_date = current_date.checked_sub_days(Days::new(15)).unwrap();

        issue_points.push(egui_plot::PlotPoint::new(
            middle_date.timestamp() as f64,
            closed.len() as f64,
        ));

        current_date = current_date.checked_add_days(Days::new(1)).unwrap();
    }

    issue_points
}

fn x_formatter(mark: GridMark, _range: &RangeInclusive<f64>) -> String {
    let step_size = TimeDelta::seconds(mark.step_size as i64);
    let mark = DateTime::from_timestamp(mark.value as i64, 0).unwrap();

    if step_size >= TimeDelta::days(365) {
        return format!("{}", mark.year());
    }

    if step_size >= TimeDelta::days(30) {
        return format!(
            "{} {}",
            &Month::try_from(mark.month() as u8).unwrap().name()[0..3],
            mark.year()
        );
    }

    return format!("{}", mark.day());
}

fn grid_spacer(input: GridInput) -> Vec<GridMark> {
    let start_time = DateTime::from_timestamp(input.bounds.0 as i64, 0).unwrap();
    let end_time = DateTime::from_timestamp(input.bounds.1 as i64, 0).unwrap();

    let base_step = TimeDelta::seconds(input.base_step_size as i64);

    let mut marks = HashMap::default();

    let grid_funcs: &[(
        TimeDelta,
        fn(DateTime<Utc>, DateTime<Utc>, &mut HashMap<i64, GridMark>),
    )] = &[
        (TimeDelta::days(1), |start, end, marks| {
            marks_by_day(start, end, 1, marks)
        }),
        (TimeDelta::days(7), |start, end, marks| {
            marks_by_day(start, end, 7, marks)
        }),
        (TimeDelta::days(30), |start, end, marks| {
            marks_by_month(start, end, 1, marks)
        }),
        (TimeDelta::days(2 * 30), |start, end, marks| {
            marks_by_month(start, end, 2, marks)
        }),
        (TimeDelta::days(6 * 30), |start, end, marks| {
            marks_by_month(start, end, 6, marks)
        }),
        (TimeDelta::days(365), |start, end, marks| {
            marks_by_year(start, end, 1, marks)
        }),
        (TimeDelta::days(365 * 3), |start, end, marks| {
            marks_by_year(start, end, 3, marks)
        }),
        (TimeDelta::days(365 * 6), |start, end, marks| {
            marks_by_year(start, end, 6, marks)
        }),
    ];

    let start_func_idx = grid_funcs
        .iter()
        .position(|(step, _)| *step >= base_step)
        .unwrap_or(grid_funcs.len() - 1);

    for &(_, func) in grid_funcs.iter().skip(start_func_idx).take(4) {
        func(start_time, end_time, &mut marks);
    }

    marks.into_values().collect()
}

fn marks_by_day(
    start_time: DateTime<chrono::Utc>,
    end_time: DateTime<chrono::Utc>,
    days: u32,
    marks: &mut HashMap<i64, GridMark>,
) {
    // Iterate day by day
    let floor_day = start_time.day0() / days * days;

    let mut current_date =
        NaiveDate::from_ymd_opt(start_time.year(), start_time.month(), floor_day + 1).unwrap();
    let mut current_month = start_time.month();

    while current_date < end_time.date_naive() {
        let timestamp = current_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        marks.insert(
            timestamp,
            GridMark {
                value: timestamp as f64,
                step_size: days as f64 * 24.0 * 60.0 * 60.0,
            },
        );

        current_date = current_date
            .checked_add_days(Days::new(days as u64))
            .unwrap();

        if current_date.month() != current_month {
            // Refloor to the first day of the month
            current_date = current_date.with_day(1).unwrap();
            current_month = current_date.month();
        }
    }
}

fn marks_by_month(
    start_time: DateTime<chrono::Utc>,
    end_time: DateTime<chrono::Utc>,
    months: u32,
    marks: &mut HashMap<i64, GridMark>,
) {
    // Iterate month by month
    let floor_month = start_time.month0() / months * months;

    let mut current_date = NaiveDate::from_ymd_opt(start_time.year(), floor_month + 1, 1).unwrap();

    while current_date < end_time.date_naive() {
        let timestamp = current_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        marks.insert(
            timestamp,
            GridMark {
                value: timestamp as f64,
                step_size: months as f64 * 30.0 * 24.0 * 60.0 * 60.0,
            },
        );

        current_date = current_date
            .checked_add_months(Months::new(months))
            .unwrap()
    }
}

fn marks_by_year(
    start_time: DateTime<chrono::Utc>,
    end_time: DateTime<chrono::Utc>,
    years: i32,
    marks: &mut HashMap<i64, GridMark>,
) {
    let floor_year = start_time.year() / years * years;

    let mut current_date = NaiveDate::from_yo_opt(floor_year, 1).unwrap();

    while current_date < end_time.date_naive() {
        let timestamp = current_date
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp();

        marks.insert(
            timestamp,
            GridMark {
                value: timestamp as f64,
                step_size: years as f64 * 365.0 * 24.0 * 60.0 * 60.0,
            },
        );

        current_date = current_date.with_year(current_date.year() + years).unwrap();
    }
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
