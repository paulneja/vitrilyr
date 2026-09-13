use std::time::Duration;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LyricLine {
    pub timestamp: Duration,
    pub text: String,
}

pub fn timestamp(value: &str) -> Option<Duration> {
    let (minutes, seconds) = value.split_once(':')?;
    if minutes.is_empty() || !minutes.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let minutes: u64 = minutes.parse().ok()?;
    let (seconds, fraction) = seconds.split_once('.').unwrap_or((seconds, ""));
    if seconds.len() != 2 || !seconds.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let seconds: u64 = seconds.parse().ok()?;
    if seconds >= 60 || fraction.len() > 3 || !fraction.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let fraction = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u64>().ok()? * 10_u64.pow(3 - fraction.len() as u32)
    };
    let ms = minutes
        .checked_mul(60_000)?
        .checked_add(seconds * 1000)?
        .checked_add(fraction)?;
    Some(Duration::from_millis(ms))
}

pub fn parse(source: &str) -> Vec<LyricLine> {
    let offset = source
        .lines()
        .find_map(|line| {
            line.trim()
                .strip_prefix("[offset:")?
                .strip_suffix(']')?
                .trim()
                .parse::<i64>()
                .ok()
        })
        .unwrap_or(0);
    let mut lines = Vec::new();
    for line in source.lines() {
        let mut rest = line.trim().trim_start_matches('\u{feff}');
        let mut times = Vec::new();
        while let Some(tail) = rest.strip_prefix('[') {
            let Some((tag, tail)) = tail.split_once(']') else {
                break;
            };
            if let Some(time) = timestamp(tag) {
                times.push(time);
            }
            rest = tail;
        }
        for time in times {
            let ms =
                (time.as_millis() as i128 + i128::from(offset)).clamp(0, u64::MAX as i128) as u64;
            lines.push(LyricLine {
                timestamp: Duration::from_millis(ms),
                text: rest.trim().into(),
            });
        }
    }
    lines.sort_by_key(|line| line.timestamp);
    // Equal timestamps represent simultaneous vocals. Keep them together.
    let mut merged: Vec<LyricLine> = Vec::new();
    for line in lines {
        if let Some(last) = merged
            .last_mut()
            .filter(|last| last.timestamp == line.timestamp)
        {
            if !line.text.is_empty() && last.text != line.text {
                if !last.text.is_empty() {
                    last.text.push_str(" / ");
                }
                last.text.push_str(&line.text);
            }
        } else {
            merged.push(line);
        }
    }
    merged
}

pub fn active(lines: &[LyricLine], position: Duration) -> Option<usize> {
    lines
        .partition_point(|line| line.timestamp <= position)
        .checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn timestamps_are_exact_and_bounded() {
        assert_eq!(timestamp("01:02.34"), Some(Duration::from_millis(62_340)));
        assert_eq!(timestamp("01:02.345"), Some(Duration::from_millis(62_345)));
        assert_eq!(timestamp("00:02"), Some(Duration::from_secs(2)));
        for bad in [
            "-1:02",
            "1:60",
            "1:2",
            "NaN",
            "1:02.1234",
            "99999999999999999999:02",
        ] {
            assert!(timestamp(bad).is_none(), "{bad}");
        }
    }
    #[test]
    fn repeated_tags_simultaneous_lines_empty_sections_and_offset() {
        let lines = parse(
            "[ar:Someone]\n[offset:-500]\n[00:02.00][00:05.000]First\n[00:05.000]Harmony\n[00:08.00]\n[bad]broken\n[00:12.00",
        );
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0].timestamp, Duration::from_millis(1500));
        assert_eq!(lines[1].text, "First / Harmony");
        assert_eq!(lines[2].text, "");
    }
    #[test]
    fn selection_handles_intro_boundaries_seeks_and_outro() {
        let lines = parse("[00:10.00]One\n[00:20.00]Two");
        for (seconds, expected) in [
            (0, None),
            (10, Some(0)),
            (19, Some(0)),
            (20, Some(1)),
            (80, Some(1)),
            (11, Some(0)),
            (1, None),
        ] {
            assert_eq!(active(&lines, Duration::from_secs(seconds)), expected);
        }
        assert_eq!(active(&[], Duration::ZERO), None);
    }
}
