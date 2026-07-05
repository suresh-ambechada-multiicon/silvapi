pub fn normalize_path_params(url: &str) -> String {
    let mut out = String::with_capacity(url.len());
    let chars: Vec<char> = url.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        match chars[i] {
            '{' if chars.get(i + 1) == Some(&'{') => {
                out.push(chars[i]);
                out.push(chars[i + 1]);
                i += 2;
                while i < chars.len() {
                    out.push(chars[i]);
                    if chars[i] == '}' && chars.get(i + 1) == Some(&'}') {
                        out.push(chars[i + 1]);
                        i += 2;
                        break;
                    }
                    i += 1;
                }
            }
            '{' => {
                if let Some((name, next)) = read_wrapped_name(&chars, i + 1, '}') {
                    out.push(':');
                    out.push_str(&name);
                    i = next;
                } else {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            '<' => {
                if let Some((name, next)) = read_wrapped_name(&chars, i + 1, '>') {
                    out.push(':');
                    out.push_str(&name);
                    i = next;
                } else {
                    out.push(chars[i]);
                    i += 1;
                }
            }
            _ => {
                out.push(chars[i]);
                i += 1;
            }
        }
    }

    out
}

pub fn extract_path_params(url: &str) -> Vec<String> {
    let normalized = normalize_path_params(url);
    let chars: Vec<char> = normalized.chars().collect();
    let mut params = Vec::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == ':' && is_path_param_start(&chars, i) {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && is_param_char(chars[end]) {
                end += 1;
            }
            if end > start {
                let name: String = chars[start..end].iter().collect();
                if !params.contains(&name) {
                    params.push(name);
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }

    params
}

pub fn extract_query_value_params(url: &str) -> Vec<String> {
    let normalized = normalize_path_params(url);
    let without_fragment = normalized
        .split_once('#')
        .map_or(normalized.as_str(), |(url, _)| url);
    let Some((_, query)) = without_fragment.split_once('?') else {
        return Vec::new();
    };

    let mut params = Vec::new();
    for pair in query.split('&') {
        let Some((_, value)) = pair.split_once('=') else {
            continue;
        };
        let Some(name) = value.strip_prefix(':') else {
            continue;
        };
        let name = name
            .chars()
            .take_while(|ch| is_param_char(*ch))
            .collect::<String>();
        if !name.is_empty() && !params.contains(&name) {
            params.push(name);
        }
    }

    params
}

pub fn replace_path_param(url: &str, key: &str, value: &str) -> (String, bool) {
    let normalized = normalize_path_params(url);
    let chars: Vec<char> = normalized.chars().collect();
    let mut out = String::with_capacity(normalized.len());
    let mut changed = false;
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == ':' && is_path_param_start(&chars, i) {
            let start = i + 1;
            let mut end = start;
            while end < chars.len() && is_param_char(chars[end]) {
                end += 1;
            }
            let name: String = chars[start..end].iter().collect();
            if name == key {
                out.push_str(value);
                changed = true;
                i = end;
                continue;
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    (out, changed)
}

fn read_wrapped_name(chars: &[char], start: usize, end_char: char) -> Option<(String, usize)> {
    let mut end = start;
    while end < chars.len() && chars[end] != end_char {
        end += 1;
    }
    if end >= chars.len() {
        return None;
    }

    let name: String = chars[start..end].iter().collect();
    if name.is_empty() || !name.chars().all(is_param_char) {
        return None;
    }

    Some((name, end + 1))
}

fn is_path_param_start(chars: &[char], colon_index: usize) -> bool {
    let next_is_name = chars
        .get(colon_index + 1)
        .map(|c| is_param_char(*c))
        .unwrap_or(false);
    if !next_is_name {
        return false;
    }

    colon_index == 0 || matches!(chars.get(colon_index.wrapping_sub(1)), Some('/'))
}

fn is_param_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

#[cfg(test)]
#[path = "path_params_tests.rs"]
mod tests;
