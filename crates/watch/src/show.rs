use serde::Deserialize;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Link {
    pub id: String,
    pub title: String,
    pub status: String,
    pub dependency_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
pub struct Shown {
    pub id: String,
    pub title: String,
    pub status: String,
    pub parent: Option<String>,
    pub description: Option<String>,
    #[serde(default)]
    pub dependencies: Vec<Link>,
    #[serde(default)]
    pub dependents: Vec<Link>,
}

pub fn parse(show: &str) -> Result<Shown, String> {
    match serde_json::from_str::<Vec<Shown>>(show) {
        Ok(mut list) if !list.is_empty() => Ok(list.swap_remove(0)),
        Ok(_) | Err(_) => {
            serde_json::from_str(show).map_err(|e: serde_json::Error| format!("br show: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE: &str = r#"{"id":"x","title":"t","status":"open","parent":"e","description":"text","dependencies":[{"id":"y","title":"u","status":"closed","dependency_type":"blocks"}]}"#;

    #[test]
    fn show_is_read_as_a_list_or_as_one_object() {
        let listed: Result<Shown, String> = parse(&format!("[{ONE}]"));
        let single: Result<Shown, String> = parse(ONE);
        assert!(
            listed == single
                && single.as_ref().is_ok_and(|s: &Shown| {
                    s.parent.as_deref() == Some("e")
                        && s.description.as_deref() == Some("text")
                        && s.dependencies.len() == 1
                        && s.dependents.is_empty()
                })
        );
    }

    #[test]
    fn a_missing_description_or_parent_is_none() {
        let bare: Shown =
            parse(r#"{"id":"x","title":"t","status":"open","parent":null,"description":null}"#)
                .unwrap();
        assert_eq!((bare.parent, bare.description), (None, None));
    }

    #[test]
    fn an_empty_or_unreadable_show_is_refused() {
        assert!(
            parse("[]").is_err_and(|e: String| e.starts_with("br show:"))
                && parse("nope").is_err_and(|e: String| e.starts_with("br show:"))
        );
    }
}
