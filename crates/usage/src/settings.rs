use eink::canvas::Port;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Settings {
    pub port: Option<Port>,
    pub level: Option<u32>,
    pub on: Option<bool>,
}

pub fn parse(text: &str) -> Settings {
    text.lines()
        .filter_map(|line: &str| line.split_once('='))
        .fold(
            Settings::default(),
            |settings: Settings, (key, value): (&str, &str)| match (key.trim(), value.trim()) {
                ("port", "left") => Settings {
                    port: Some(Port::Left),
                    ..settings
                },
                ("port", "right") => Settings {
                    port: Some(Port::Right),
                    ..settings
                },
                ("light", level) => Settings {
                    level: level.parse::<u32>().ok().or(settings.level),
                    ..settings
                },
                ("on", "yes") => Settings {
                    on: Some(true),
                    ..settings
                },
                ("on", "no") => Settings {
                    on: Some(false),
                    ..settings
                },
                _ => settings,
            },
        )
}

pub fn render(port: Port, level: u32, on: bool) -> String {
    let side: &str = match port {
        Port::Left => "left",
        Port::Right => "right",
    };
    format!(
        "port={side}\nlight={level}\non={}\n",
        if on { "yes" } else { "no" }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_saved_reads_as_nothing_known() {
        assert_eq!(parse(""), Settings::default());
    }

    #[test]
    fn what_is_written_reads_back() {
        let both: [Settings; 2] = [
            parse(&render(Port::Left, 9, false)),
            parse(&render(Port::Right, 25, true)),
        ];
        let expected: [Settings; 2] = [
            Settings {
                port: Some(Port::Left),
                level: Some(9),
                on: Some(false),
            },
            Settings {
                port: Some(Port::Right),
                level: Some(25),
                on: Some(true),
            },
        ];
        assert_eq!(both, expected);
    }

    #[test]
    fn unknown_keys_and_bad_values_are_ignored() {
        let read: Settings =
            parse("port=up\nlight=bright\non=maybe\ncolour=red\nlight=7\nnonsense\n");
        assert_eq!(
            read,
            Settings {
                port: None,
                level: Some(7),
                on: None
            }
        );
    }

    #[test]
    fn a_bad_value_keeps_an_earlier_good_one() {
        assert_eq!(parse("light=7\nlight=x\n").level, Some(7));
    }
}
