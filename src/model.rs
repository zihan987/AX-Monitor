#[derive(Clone, Debug)]
pub struct Meter {
    pub title: &'static str,
    pub percent: Option<f64>,
    pub value: String,
    pub detail: String,
}

impl Meter {
    pub fn unavailable(title: &'static str, detail: impl Into<String>) -> Self {
        Self {
            title,
            percent: None,
            value: "N/A".to_string(),
            detail: detail.into(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub meters: Vec<Meter>,
}

pub fn clamp_percent(value: f64) -> f64 {
    value.clamp(0.0, 100.0)
}

pub fn format_percent(value: Option<f64>) -> String {
    match value {
        Some(value) => format!("{value:.1}%"),
        None => "N/A".to_string(),
    }
}

pub fn format_size_mb(value_mb: f64) -> String {
    if value_mb >= 1024.0 {
        format!("{:.2} GB", value_mb / 1024.0)
    } else {
        format!("{value_mb:.0} MB")
    }
}

