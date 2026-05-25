use serde::Serialize;
use tabled::{Table, Tabled};

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
pub enum OutputFormat {
    #[default]
    Table,
    Json,
}

pub fn render_list<T: Tabled + Serialize>(items: &[T], format: OutputFormat) -> String {
    match format {
        OutputFormat::Table => {
            if items.is_empty() {
                "(no results)".to_string()
            } else {
                Table::new(items).to_string()
            }
        }
        OutputFormat::Json => {
            serde_json::to_string_pretty(items).unwrap_or_else(|_| "[]".to_string())
        }
    }
}

pub fn render_single<T: Serialize>(item: &T, _format: OutputFormat) -> String {
    serde_json::to_string_pretty(item).unwrap_or_else(|_| "{}".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Serialize;
    use tabled::Tabled;

    #[derive(Debug, Tabled, Serialize)]
    struct TestRow {
        name: String,
        status: String,
    }

    #[test]
    fn render_list_table_empty() {
        let items: Vec<TestRow> = vec![];
        let out = render_list(&items, OutputFormat::Table);
        assert_eq!(out, "(no results)");
    }

    #[test]
    fn render_list_table_has_header() {
        let items = vec![TestRow {
            name: "drone-1".to_string(),
            status: "ready".to_string(),
        }];
        let out = render_list(&items, OutputFormat::Table);
        assert!(out.contains("name"));
        assert!(out.contains("drone-1"));
    }

    #[test]
    fn render_list_json() {
        let items = vec![TestRow {
            name: "drone-1".to_string(),
            status: "ready".to_string(),
        }];
        let out = render_list(&items, OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        let arr = parsed.as_array().expect("is array");
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["name"], "drone-1");
    }

    #[test]
    fn render_single_json() {
        let item = TestRow {
            name: "drone-1".to_string(),
            status: "ready".to_string(),
        };
        let out = render_single(&item, OutputFormat::Json);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["name"], "drone-1");
    }
}
