use std::ops::RangeInclusive;

use chrono::{DateTime, Datelike, Days, Month, Months, NaiveDate, TimeDelta, Utc};
use eframe::egui::ahash::HashMap;
use egui_plot::{GridInput, GridMark};

pub fn label_formatter(name: &str, point: &egui_plot::PlotPoint) -> String {
    let date = chrono::DateTime::from_timestamp(point.x as i64, 0)
        .unwrap()
        .date_naive();

    if name == "" {
        return format!("{date}");
    }

    format!("{name} on {date}: {}", point.y)
}

pub fn x_formatter(mark: GridMark, _range: &RangeInclusive<f64>) -> String {
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

pub fn grid_spacer(input: GridInput) -> Vec<GridMark> {
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
