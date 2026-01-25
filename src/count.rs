use memchr::memmem::Finder;

const CHUNK_SIZE: usize = 1024 * 1024;
const PARALLEL_THRESHOLD: usize = 512 * 1024;

pub fn count_lines(data: &[u8]) -> usize {
    if data.len() < PARALLEL_THRESHOLD {
        return memchr::memchr_iter(b'\n', data).count();
    }

    use rayon::prelude::*;
    data.par_chunks(CHUNK_SIZE)
        .map(|chunk| memchr::memchr_iter(b'\n', chunk).count())
        .sum()
}

pub fn count_all_words(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return count_words_in_chunk(data);
    }

    use rayon::prelude::*;

    let chunk_boundaries = find_utf8_chunk_boundaries(data, CHUNK_SIZE);

    let count: usize = chunk_boundaries
        .par_windows(2)
        .map(|window| {
            let chunk = &data[window[0]..window[1]];
            count_words_in_chunk(chunk)
        })
        .sum();

    let mut overcounted = 0;
    for window in chunk_boundaries.windows(2) {
        let boundary = window[1];
        if boundary > 0 && boundary < data.len() {
            let prev_byte = data[boundary - 1];
            let curr_byte = data[boundary];
            if !is_whitespace_byte(prev_byte) && !is_whitespace_byte(curr_byte) {
                overcounted += 1;
            }
        }
    }

    count.saturating_sub(overcounted)
}

fn find_utf8_chunk_boundaries(data: &[u8], chunk_size: usize) -> Vec<usize> {
    let mut boundaries = vec![0];
    let mut pos = chunk_size;

    while pos < data.len() {
        pos = find_utf8_boundary(data, pos);
        boundaries.push(pos);
        pos += chunk_size;
    }

    boundaries.push(data.len());
    boundaries
}

fn find_utf8_boundary(data: &[u8], pos: usize) -> usize {
    if pos >= data.len() {
        return data.len();
    }

    let mut p = pos;
    while p < data.len() && (data[p] & 0xC0) == 0x80 {
        p += 1;
    }
    p
}

#[inline]
fn is_whitespace_byte(byte: u8) -> bool {
    byte.is_ascii_whitespace()
}

#[inline]
fn is_unicode_whitespace(c: char) -> bool {
    c.is_whitespace()
}

#[inline]
fn count_words_in_chunk(chunk: &[u8]) -> usize {
    if let Ok(text) = std::str::from_utf8(chunk) {
        let mut count = 0;
        let mut in_word = false;

        for c in text.chars() {
            if is_unicode_whitespace(c) {
                in_word = false;
            } else if !in_word {
                count += 1;
                in_word = true;
            }
        }

        count
    } else {
        let mut count = 0;
        let mut in_word = false;

        for &byte in chunk {
            if byte.is_ascii_whitespace() {
                in_word = false;
            } else if !in_word {
                count += 1;
                in_word = true;
            }
        }

        count
    }
}

pub fn count_pattern(data: &[u8], pattern: &[u8]) -> usize {
    if data.is_empty() || pattern.is_empty() {
        return 0;
    }

    let finder = Finder::new(pattern);

    if data.len() < PARALLEL_THRESHOLD {
        return finder.find_iter(data).count();
    }

    use rayon::prelude::*;

    let overlap = pattern.len() - 1;
    let num_chunks = data.len().div_ceil(CHUNK_SIZE);

    let count: usize = (0..num_chunks)
        .into_par_iter()
        .map(|i| {
            let start = i * CHUNK_SIZE;
            let end = ((i + 1) * CHUNK_SIZE).min(data.len());
            let chunk = &data[start..end];
            finder.find_iter(chunk).count()
        })
        .sum();

    let mut missed = 0;
    for i in 1..num_chunks {
        let prev_chunk_end = i * CHUNK_SIZE;
        let boundary_start = prev_chunk_end.saturating_sub(overlap);
        let boundary_end = (prev_chunk_end + overlap).min(data.len());

        let boundary = &data[boundary_start..boundary_end];
        missed += finder.find_iter(boundary).count();
    }

    count + missed
}

pub fn count_chars(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return std::str::from_utf8(data)
            .map(|s| s.chars().count())
            .unwrap_or(data.len());
    }

    use rayon::prelude::*;

    let chunk_boundaries = find_utf8_chunk_boundaries(data, CHUNK_SIZE);

    chunk_boundaries
        .par_windows(2)
        .map(|window| {
            let chunk = &data[window[0]..window[1]];
            std::str::from_utf8(chunk)
                .map(|s| s.chars().count())
                .unwrap_or(chunk.len())
        })
        .sum()
}

pub fn max_line_length(data: &[u8]) -> usize {
    if data.is_empty() {
        return 0;
    }

    if data.len() < PARALLEL_THRESHOLD {
        return max_line_length_sequential(data);
    }

    let mut line_start = 0;
    let mut max_len = 0;

    for (i, &byte) in data.iter().enumerate() {
        if byte == b'\n' {
            let mut line_end = i;
            if line_end > line_start && data[line_end - 1] == b'\r' {
                line_end -= 1;
            }
            let line_len = line_end - line_start;
            max_len = max_len.max(line_len);
            line_start = i + 1;
        }
    }

    if line_start < data.len() {
        let mut line_end = data.len();
        if line_end > line_start && data[line_end - 1] == b'\r' {
            line_end -= 1;
        }
        let line_len = line_end - line_start;
        max_len = max_len.max(line_len);
    }

    max_len
}

#[inline]
fn max_line_length_sequential(data: &[u8]) -> usize {
    let mut max_len = 0;
    let mut current_len = 0;

    for &byte in data {
        if byte == b'\n' {
            max_len = max_len.max(current_len);
            current_len = 0;
        } else if byte == b'\r' {
        } else {
            current_len += 1;
        }
    }

    max_len.max(current_len)
}

pub fn is_binary(data: &[u8]) -> bool {
    let sample_size = data.len().min(8192);
    let sample = &data[..sample_size];

    for &byte in sample {
        if byte == 0 || (byte < 32 && byte != b'\n' && byte != b'\r' && byte != b'\t') {
            return true;
        }
    }
    false
}

use std::collections::{HashMap, HashSet};

pub fn count_unique_words(data: &[u8]) -> usize {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return 0,
    };

    if data.len() < PARALLEL_THRESHOLD {
        let words: HashSet<&str> = text
            .split(|c: char| c.is_whitespace())
            .filter(|w| !w.is_empty())
            .collect();
        return words.len();
    }

    use rayon::prelude::*;

    let lines: Vec<&str> = text.lines().collect();
    let chunk_size = (lines.len() / rayon::current_num_threads()).max(1000);

    let local_sets: Vec<HashSet<&str>> = lines
        .par_chunks(chunk_size)
        .map(|chunk_lines| {
            let mut local_set = HashSet::new();
            for line in chunk_lines {
                for word in line.split(|c: char| c.is_whitespace()) {
                    if !word.is_empty() {
                        local_set.insert(word);
                    }
                }
            }
            local_set
        })
        .collect();

    let mut final_set = HashSet::new();
    for set in local_sets {
        final_set.extend(set);
    }

    final_set.len()
}

pub struct Statistics {
    pub mean_line_length: f64,
    pub median_line_length: usize,
    pub std_dev: f64,
    pub min_line_length: usize,
    pub max_line_length: usize,
    pub empty_lines: usize,
}

pub fn calculate_statistics(data: &[u8]) -> Statistics {
    let mut line_lengths = Vec::new();
    let mut current_len = 0;
    let mut empty_lines = 0;

    for &byte in data {
        if byte == b'\n' {
            line_lengths.push(current_len);
            if current_len == 0 {
                empty_lines += 1;
            }
            current_len = 0;
        } else if byte == b'\r' {
        } else {
            current_len += 1;
        }
    }

    if current_len > 0 || !data.is_empty() {
        line_lengths.push(current_len);
        if current_len == 0 {
            empty_lines += 1;
        }
    }

    if line_lengths.is_empty() {
        return Statistics {
            mean_line_length: 0.0,
            median_line_length: 0,
            std_dev: 0.0,
            min_line_length: 0,
            max_line_length: 0,
            empty_lines: 0,
        };
    }

    let sum: usize = line_lengths.iter().sum();
    let mean = sum as f64 / line_lengths.len() as f64;

    let variance: f64 = line_lengths
        .iter()
        .map(|&len| {
            let diff = len as f64 - mean;
            diff * diff
        })
        .sum::<f64>()
        / line_lengths.len() as f64;

    let std_dev = variance.sqrt();

    line_lengths.sort_unstable();
    let median = if line_lengths.len() % 2 == 0 {
        let mid = line_lengths.len() / 2;
        (line_lengths[mid - 1] + line_lengths[mid]) / 2
    } else {
        line_lengths[line_lengths.len() / 2]
    };

    let min = *line_lengths.iter().min().unwrap_or(&0);
    let max = *line_lengths.iter().max().unwrap_or(&0);

    Statistics {
        mean_line_length: mean,
        median_line_length: median,
        std_dev,
        min_line_length: min,
        max_line_length: max,
        empty_lines,
    }
}

pub fn generate_histogram(data: &[u8]) -> HashMap<usize, usize> {
    let mut histogram = HashMap::new();
    let mut current_len = 0;

    for &byte in data {
        if byte == b'\n' {
            let bucket = (current_len / 10) * 10;
            *histogram.entry(bucket).or_insert(0) += 1;
            current_len = 0;
        } else if byte == b'\r' {
        } else {
            current_len += 1;
        }
    }

    if current_len > 0 {
        let bucket = (current_len / 10) * 10;
        *histogram.entry(bucket).or_insert(0) += 1;
    }

    histogram
}

pub fn filter_code_comments(data: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return data.to_vec(),
    };

    let mut result = Vec::new();
    let mut in_multiline_c_comment = false;
    let mut in_python_docstring = false;
    let mut docstring_marker: &str = "";

    for line in text.lines() {
        let trimmed = line.trim();

        if in_multiline_c_comment {
            if trimmed.contains("*/") {
                in_multiline_c_comment = false;
            }
            continue;
        }

        if in_python_docstring {
            if trimmed.ends_with(docstring_marker)
                || (trimmed.contains(docstring_marker) && !trimmed.starts_with(docstring_marker))
            {
                in_python_docstring = false;
            }
            continue;
        }

        if trimmed.starts_with("/*") {
            in_multiline_c_comment = true;
            if trimmed.ends_with("*/") && trimmed.len() > 3 {
                in_multiline_c_comment = false;
            }
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("\"\"\"") {
            docstring_marker = "\"\"\"";
            in_python_docstring = !rest.contains("\"\"\"");
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("'''") {
            docstring_marker = "'''";
            in_python_docstring = !rest.contains("'''");
            continue;
        }

        if trimmed.is_empty()
            || trimmed.starts_with("//")
            || trimmed.starts_with('#')
            || trimmed.starts_with("--")
        {
            continue;
        }

        result.extend_from_slice(line.as_bytes());
        result.push(b'\n');
    }

    result
}

pub fn filter_markdown_code(data: &[u8]) -> Vec<u8> {
    let text = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return data.to_vec(),
    };

    let mut result = Vec::new();
    let mut in_code_block = false;

    for line in text.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }

        if in_code_block {
            continue;
        }

        let filtered_line = filter_inline_code(line);
        result.extend_from_slice(filtered_line.as_bytes());
        result.push(b'\n');
    }

    result
}

fn filter_inline_code(line: &str) -> String {
    let mut result = String::new();
    let mut in_code = false;

    for c in line.chars() {
        if c == '`' {
            in_code = !in_code;
        } else if !in_code {
            result.push(c);
        }
    }

    result
}

pub fn decode_to_utf8(data: &[u8], encoding_name: Option<&str>) -> Vec<u8> {
    use chardetng::EncodingDetector;
    use encoding_rs::Encoding;

    let encoding = if let Some(name) = encoding_name {
        Encoding::for_label(name.as_bytes()).unwrap_or(encoding_rs::UTF_8)
    } else {
        let mut detector = EncodingDetector::new();
        detector.feed(data, true);
        detector.guess(None, true)
    };

    if encoding == encoding_rs::UTF_8 {
        return data.to_vec();
    }

    let (decoded, _, _) = encoding.decode(data);
    decoded.into_owned().into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_lines_empty() {
        assert_eq!(count_lines(b""), 0);
    }

    #[test]
    fn test_count_lines_single() {
        assert_eq!(count_lines(b"hello\n"), 1);
    }

    #[test]
    fn test_count_lines_multiple() {
        assert_eq!(count_lines(b"line1\nline2\nline3\n"), 3);
    }

    #[test]
    fn test_count_lines_no_trailing_newline() {
        assert_eq!(count_lines(b"line1\nline2"), 1);
    }

    #[test]
    fn test_count_words_empty() {
        assert_eq!(count_all_words(b""), 0);
    }

    #[test]
    fn test_count_words_single() {
        assert_eq!(count_all_words(b"hello"), 1);
    }

    #[test]
    fn test_count_words_multiple() {
        assert_eq!(count_all_words(b"hello world foo bar"), 4);
    }

    #[test]
    fn test_count_words_multiple_spaces() {
        assert_eq!(count_all_words(b"hello    world"), 2);
    }

    #[test]
    fn test_count_words_newlines() {
        assert_eq!(count_all_words(b"hello\nworld\nfoo"), 3);
    }

    #[test]
    fn test_count_words_mixed_whitespace() {
        assert_eq!(count_all_words(b"hello\t\nworld  \r\nfoo"), 3);
    }

    #[test]
    fn test_count_words_unicode_whitespace() {
        let text = "hello\u{00A0}world";
        assert_eq!(count_all_words(text.as_bytes()), 2);

        let text2 = "hello\u{2003}world";
        assert_eq!(count_all_words(text2.as_bytes()), 2);
    }

    #[test]
    fn test_count_pattern_empty_data() {
        assert_eq!(count_pattern(b"", b"test"), 0);
    }

    #[test]
    fn test_count_pattern_empty_pattern() {
        assert_eq!(count_pattern(b"test", b""), 0);
    }

    #[test]
    fn test_count_pattern_single_occurrence() {
        assert_eq!(count_pattern(b"hello world", b"world"), 1);
    }

    #[test]
    fn test_count_pattern_multiple_occurrences() {
        assert_eq!(count_pattern(b"foo bar foo baz foo", b"foo"), 3);
    }

    #[test]
    fn test_count_pattern_non_overlapping() {
        assert_eq!(count_pattern(b"aaa", b"aa"), 1);
        assert_eq!(count_pattern(b"aaaa", b"aa"), 2);
    }

    #[test]
    fn test_count_pattern_no_match() {
        assert_eq!(count_pattern(b"hello world", b"xyz"), 0);
    }

    #[test]
    fn test_count_pattern_byte_pattern() {
        assert_eq!(count_pattern(b"a\nb\nc\n", b"\n"), 3);
    }

    #[test]
    fn test_large_data_parallel() {
        let large_text = "word ".repeat(200_000);
        let bytes = large_text.as_bytes();

        assert_eq!(count_all_words(bytes), 200_000);

        let large_lines = b"line\n".repeat(200_000);
        assert_eq!(count_lines(&large_lines), 200_000);
    }

    #[test]
    fn test_chunk_boundary_words() {
        let chunk_size = CHUNK_SIZE;
        let mut data = vec![b'a'; chunk_size - 1];
        data.push(b'b');
        data.push(b'c');

        assert_eq!(count_all_words(&data), 1);

        data[chunk_size - 1] = b' ';
        assert_eq!(count_all_words(&data), 2);
    }

    #[test]
    fn test_chunk_boundary_pattern() {
        let chunk_size = CHUNK_SIZE;
        let pattern = b"boundary";
        let mut data = vec![b'x'; chunk_size - 4];
        data.extend_from_slice(pattern);
        data.extend_from_slice(b"yyyyyy");

        assert_eq!(count_pattern(&data, pattern), 1);
    }

    #[test]
    fn test_count_chars_empty() {
        assert_eq!(count_chars(b""), 0);
    }

    #[test]
    fn test_count_chars_ascii() {
        assert_eq!(count_chars(b"hello world"), 11);
    }

    #[test]
    fn test_count_chars_utf8() {
        assert_eq!(count_chars("hello 世界".as_bytes()), 8);
        assert_eq!(count_chars("🦀 Rust".as_bytes()), 6);
    }

    #[test]
    fn test_count_chars_vs_bytes() {
        let text = "café";
        assert_eq!(count_chars(text.as_bytes()), 4);
        assert_eq!(text.as_bytes().len(), 5);
    }

    #[test]
    fn test_max_line_length_empty() {
        assert_eq!(max_line_length(b""), 0);
    }

    #[test]
    fn test_max_line_length_single_line() {
        assert_eq!(max_line_length(b"hello"), 5);
    }

    #[test]
    fn test_max_line_length_multiple_lines() {
        assert_eq!(max_line_length(b"hi\nhello\nbye"), 5);
    }

    #[test]
    fn test_max_line_length_trailing_newline() {
        assert_eq!(max_line_length(b"hello\nworld\n"), 5);
    }

    #[test]
    fn test_max_line_length_empty_lines() {
        assert_eq!(max_line_length(b"\n\nhello\n\n"), 5);
    }

    #[test]
    fn test_max_line_length_crlf() {
        assert_eq!(max_line_length(b"hello\r\nworld\r\n"), 5);
        assert_eq!(max_line_length(b"hi\r\nhello\r\nbye\r\n"), 5);
    }

    #[test]
    fn test_max_line_length_mixed_endings() {
        assert_eq!(max_line_length(b"hello\nworld\r\nfoo\n"), 5);
    }

    #[test]
    fn test_filter_code_c_style_single_line() {
        let input = b"// this is a comment\nint x = 5;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"int x = 5;\n");
    }

    #[test]
    fn test_filter_code_c_style_multiline() {
        let input = b"/* multiline\ncomment */\nint x = 5;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"int x = 5;\n");
    }

    #[test]
    fn test_filter_code_hash_comments() {
        let input = b"# Python comment\nprint('hello')\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"print('hello')\n");
    }

    #[test]
    fn test_filter_code_sql_comments() {
        let input = b"-- SQL comment\nSELECT * FROM users;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"SELECT * FROM users;\n");
    }

    #[test]
    fn test_filter_code_python_docstring() {
        let input = b"\"\"\"\nThis is a docstring\n\"\"\"\ndef foo():\n    pass\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"def foo():\n    pass\n");
    }

    #[test]
    fn test_filter_code_empty_lines() {
        let input = b"int x = 5;\n\nint y = 10;\n";
        let output = filter_code_comments(input);
        assert_eq!(output, b"int x = 5;\nint y = 10;\n");
    }

    #[test]
    fn test_filter_markdown_code_block() {
        let input = b"Some text\n```rust\nlet x = 5;\n```\nMore text\n";
        let output = filter_markdown_code(input);
        assert!(String::from_utf8_lossy(&output).contains("Some text"));
        assert!(String::from_utf8_lossy(&output).contains("More text"));
        assert!(!String::from_utf8_lossy(&output).contains("let x = 5"));
    }

    #[test]
    fn test_filter_markdown_inline_code() {
        let input = b"Use the `println!` macro\n";
        let output = filter_markdown_code(input);
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("Use the"));
        assert!(output_str.contains("macro"));
        assert!(!output_str.contains("println!"));
    }

    #[test]
    fn test_filter_markdown_multiple_blocks() {
        let input = b"Intro\n```\ncode1\n```\nMiddle\n```\ncode2\n```\nEnd\n";
        let output = filter_markdown_code(input);
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("Intro"));
        assert!(output_str.contains("Middle"));
        assert!(output_str.contains("End"));
        assert!(!output_str.contains("code1"));
        assert!(!output_str.contains("code2"));
    }

    #[test]
    fn test_unique_words_basic() {
        let input = b"hello world hello foo world bar";
        assert_eq!(count_unique_words(input), 4);
    }

    #[test]
    fn test_unique_words_empty() {
        assert_eq!(count_unique_words(b""), 0);
    }

    #[test]
    fn test_unique_words_all_same() {
        let input = b"word word word word word";
        assert_eq!(count_unique_words(input), 1);
    }

    #[test]
    fn test_utf8_boundary_detection() {
        let text = "hello 世界 test";
        let data = text.as_bytes();

        let boundary = find_utf8_boundary(data, 7);
        assert!(std::str::from_utf8(&data[boundary..]).is_ok());
    }

    #[test]
    fn test_decode_utf8_passthrough() {
        let input = "hello world".as_bytes();
        let output = decode_to_utf8(input, Some("utf-8"));
        assert_eq!(output, input);
    }

    #[test]
    fn test_decode_autodetect_utf8() {
        let input = "hello 世界".as_bytes();
        let output = decode_to_utf8(input, None);
        assert_eq!(output, input);
    }
}
