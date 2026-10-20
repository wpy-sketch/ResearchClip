//! Bounded local bibliographic interchange. No HTML, LaTeX or model output is executed.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
pub const MAX_BIB_BYTES: usize = 1_048_576;
pub type Result<T> = std::result::Result<T, String>;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceInput {
    pub title: String,
    pub authors: Vec<String>,
    pub year: Option<i32>,
    pub journal_or_conference: String,
    pub doi: String,
    pub url: String,
    pub bibtex_key: String,
    pub bibtex_entry: String,
    pub verified_source: String,
    pub source_metadata: Value,
}
pub fn normalize_doi(input: &str) -> Result<String> {
    let raw = input.trim();
    if raw.is_empty() {
        return Ok(String::new());
    }
    let lower = raw.to_ascii_lowercase();
    let offset = [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "doi:",
    ]
    .iter()
    .find(|p| lower.starts_with(**p))
    .map_or(0, |p| p.len());
    let value = raw[offset..].trim().to_ascii_lowercase();
    let (prefix, suffix) = value.split_once('/').ok_or("DOI 格式应为 10.xxxx/xxx。")?;
    if !prefix.starts_with("10.")
        || !(4..=9).contains(&prefix[3..].len())
        || !prefix[3..].bytes().all(|c| c.is_ascii_digit())
        || suffix.is_empty()
        || value.len() > 512
        || value.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        return Err("DOI 格式无效，请检查空格、前缀与长度。".into());
    }
    Ok(value)
}
pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 100
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"_:-".contains(&b))
}
fn text(value: &str, max: usize) -> bool {
    value.chars().count() <= max
        && !value
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
}
pub fn validate(v: &ReferenceInput) -> Result<()> {
    if v.title.trim().is_empty()
        || !text(&v.title, 2000)
        || v.authors.len() > 200
        || v.authors
            .iter()
            .any(|s| s.trim().is_empty() || !text(s, 500))
        || v.year.is_some_and(|y| !(1..=9999).contains(&y))
        || !text(&v.journal_or_conference, 2000)
        || !valid_key(&v.bibtex_key)
    {
        return Err("请检查标题、作者、年份（1–9999）和引用键（字母、数字、_、:、-）。".into());
    }
    normalize_doi(&v.doi)?;
    if !v.url.is_empty() {
        let u = reqwest::Url::parse(&v.url).map_err(|_| "文献 URL 格式无效。")?;
        if !matches!(u.scheme(), "http" | "https")
            || !u.username().is_empty()
            || u.password().is_some()
            || v.url.len() > 2048
        {
            return Err("文献 URL 仅允许不含凭据的 HTTP(S) 地址。".into());
        }
    }
    if !["manual", "crossref", "bibtex", "edited"].contains(&v.verified_source.as_str())
        || v.source_metadata.to_string().len() > 262144
        || v.bibtex_entry.len() > 65536
    {
        return Err("文献来源或 BibTeX 大小无效。".into());
    }
    Ok(())
}
fn escape(value: &str) -> String {
    value
        .replace('\\', "\\textbackslash{}")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('%', "\\%")
        .replace('&', "\\&")
        .replace('#', "\\#")
        .replace('_', "\\_")
        .replace('$', "\\$")
        .replace('\n', " ")
}
#[derive(Debug)]
struct Entry {
    kind: String,
    key: String,
    fields: BTreeMap<String, String>,
    raw: String,
}
struct Parser<'a> {
    s: &'a str,
    p: usize,
}
impl<'a> Parser<'a> {
    fn byte(&self) -> Option<u8> {
        self.s.as_bytes().get(self.p).copied()
    }
    fn ws(&mut self) {
        loop {
            while self.byte().is_some_and(|c| c.is_ascii_whitespace()) {
                self.p += 1;
            }
            if self.byte() == Some(b'%') {
                while self.byte().is_some_and(|c| c != b'\n') {
                    self.p += 1;
                }
            } else {
                break;
            }
        }
    }
    fn word(&mut self) -> String {
        let start = self.p;
        while self
            .byte()
            .is_some_and(|c| c.is_ascii_alphanumeric() || b"_-:".contains(&c))
        {
            self.p += 1;
        }
        self.s[start..self.p].to_owned()
    }
    fn expect(&mut self, b: u8) -> Result<()> {
        self.ws();
        if self.byte() != Some(b) {
            return Err("BibTeX 语法不完整；未导入任何记录。".into());
        }
        self.p += 1;
        Ok(())
    }
    fn value(&mut self) -> Result<String> {
        self.ws();
        let start = self.p;
        match self.byte() {
            Some(b'{') => {
                self.p += 1;
                let content = self.p;
                let mut depth = 1;
                while let Some(c) = self.byte() {
                    self.p += 1;
                    if c == b'\\' {
                        if self.byte().is_some() {
                            let ch = self.s[self.p..].chars().next().unwrap();
                            self.p += ch.len_utf8();
                        }
                    } else if c == b'{' {
                        depth += 1;
                        if depth > 64 {
                            return Err("BibTeX 嵌套超过64层。".into());
                        }
                    } else if c == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            return Ok(self.s[content..self.p - 1].into());
                        }
                    }
                }
                Err("BibTeX 花括号未闭合。".into())
            }
            Some(b'"') => {
                self.p += 1;
                let content = self.p;
                let mut depth = 0i32;
                while let Some(c) = self.byte() {
                    self.p += 1;
                    if c == b'\\' {
                        if self.byte().is_some() {
                            let ch = self.s[self.p..].chars().next().unwrap();
                            self.p += ch.len_utf8();
                        }
                    } else if c == b'{' {
                        depth += 1;
                    } else if c == b'}' {
                        depth -= 1;
                        if depth < 0 {
                            return Err("BibTeX 引号中括号不平衡。".into());
                        }
                    } else if c == b'"' && depth == 0 {
                        return Ok(self.s[content..self.p - 1].into());
                    }
                }
                Err("BibTeX 引号未闭合。".into())
            }
            Some(c) if c.is_ascii_digit() => {
                while self.byte().is_some_and(|c| c.is_ascii_digit()) {
                    self.p += 1;
                }
                Ok(self.s[start..self.p].into())
            }
            _ => Err(
                "此导入器仅接受花括号、引号或数字字段；请先展开 @string 宏及 # 拼接表达式。".into(),
            ),
        }
    }
    fn entries(&mut self) -> Result<Vec<Entry>> {
        let mut items = vec![];
        loop {
            self.ws();
            if self.byte().is_none() {
                break;
            }
            let start = self.p;
            self.expect(b'@')?;
            let kind = self.word().to_ascii_lowercase();
            if kind.is_empty() {
                return Err("BibTeX 类型缺失。".into());
            }
            self.ws();
            let close = match self.byte() {
                Some(b'{') => b'}',
                Some(b'(') => b')',
                _ => return Err("BibTeX 记录必须用括号包围。".into()),
            };
            self.p += 1;
            if ["string", "preamble", "comment"].contains(&kind.as_str()) {
                return Err(
                    "请移除 @string、@preamble、@comment 记录或先展开；未执行任何内容。".into(),
                );
            }
            self.ws();
            let key = self.word();
            if !valid_key(&key) {
                return Err("BibTeX 引用键无效。".into());
            }
            self.expect(b',')?;
            let mut fields = BTreeMap::new();
            loop {
                self.ws();
                if self.byte() == Some(close) {
                    self.p += 1;
                    break;
                }
                let field = self.word().to_ascii_lowercase();
                if field.is_empty() {
                    return Err("BibTeX 字段名称缺失。".into());
                }
                self.expect(b'=')?;
                let value = self.value()?;
                if fields.insert(field, value).is_some() {
                    return Err("BibTeX 含重复字段。".into());
                }
                self.ws();
                if self.byte() == Some(b',') {
                    self.p += 1;
                } else if self.byte() != Some(close) {
                    return Err("BibTeX 不支持宏或拼接；请导出展开后的记录。".into());
                }
            }
            let raw = self.s[start..self.p].to_owned();
            if raw.len() > 65536 || items.len() >= 200 {
                return Err("单条最多64 KiB，一次最多200条记录。".into());
            }
            items.push(Entry {
                kind,
                key,
                fields,
                raw,
            });
        }
        if items.is_empty() {
            return Err("未找到 BibTeX 记录。".into());
        }
        Ok(items)
    }
}
fn entries(s: &str) -> Result<Vec<Entry>> {
    if s.len() > MAX_BIB_BYTES {
        return Err("BibTeX 文件超过1 MiB。".into());
    }
    Parser {
        s: s.trim_start_matches('\u{feff}'),
        p: 0,
    }
    .entries()
}
// Split author separators only at brace depth zero, preserving corporate authors.
fn authors(s: &str) -> Vec<String> {
    let mut depth = 0;
    let mut start = 0;
    let mut out = vec![];
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if c == '\\' {
            escaped = true;
            continue;
        }
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;
        }
        if depth == 0 && s[i..].starts_with(" and ") {
            out.push(s[start..i].trim().into());
            start = i + 5;
        }
    }
    if !s[start..].trim().is_empty() {
        out.push(s[start..].trim().into());
    }
    out
}
pub fn import_bib(s: &str) -> Result<Vec<ReferenceInput>> {
    entries(s)?
        .into_iter()
        .map(|e| {
            let field = |name: &str| e.fields.get(name).cloned().unwrap_or_default();
            let year = field("year");
            let year = if year.is_empty() {
                None
            } else {
                Some(
                    year.parse()
                        .map_err(|_| "BibTeX 年份必须为1–9999的整数；请手工修正后导入。")?,
                )
            };
            let item = ReferenceInput {
                title: field("title"),
                authors: authors(&field("author")),
                year,
                journal_or_conference: if e.fields.contains_key("journal") {
                    field("journal")
                } else {
                    field("booktitle")
                },
                doi: normalize_doi(&field("doi"))?,
                url: field("url"),
                bibtex_key: e.key.clone(),
                bibtex_entry: e.raw.clone(),
                verified_source: "bibtex".into(),
                source_metadata: serde_json::json!({"importedBibtex":e.raw}),
            };
            validate(&item)?;
            Ok(item)
        })
        .collect()
}
pub fn bibtex(v: &ReferenceInput) -> Result<String> {
    validate(v)?;
    let previous = if v.bibtex_entry.trim().is_empty() {
        None
    } else {
        let mut parsed = import_bib(&v.bibtex_entry)?;
        if parsed.len() != 1 {
            return Err("单篇文献只能包含一条 BibTeX。".into());
        }
        Some(parsed.remove(0))
    };
    if let Some(p) = &previous
        && p.title == v.title
        && p.authors == v.authors
        && p.year == v.year
        && p.journal_or_conference == v.journal_or_conference
        && p.doi == normalize_doi(&v.doi)?
        && p.url == v.url
        && p.bibtex_key == v.bibtex_key
    {
        return Ok(v.bibtex_entry.clone());
    }
    let prior = if previous.is_some() {
        entries(&v.bibtex_entry)?.into_iter().next()
    } else {
        None
    };
    let kind = prior
        .as_ref()
        .map_or("misc", |e| e.kind.as_str())
        .to_owned();
    let mut fields = prior.map_or_else(BTreeMap::new, |e| e.fields);
    // Updating a different field must not escape existing TeX, corporate braces,
    // or math source a second time. Keep their original field values verbatim.
    let mut update = |key: &str, unchanged: bool, value: Option<String>| {
        if unchanged && fields.contains_key(key) {
            return;
        }
        if let Some(value) = value {
            fields.insert(key.into(), value);
        } else {
            fields.remove(key);
        }
    };
    update(
        "title",
        previous.as_ref().is_some_and(|p| p.title == v.title),
        Some(escape(&v.title)),
    );
    update(
        "author",
        previous.as_ref().is_some_and(|p| p.authors == v.authors),
        (!v.authors.is_empty()).then(|| {
            v.authors
                .iter()
                .map(|a| escape(a))
                .collect::<Vec<_>>()
                .join(" and ")
        }),
    );
    update(
        "year",
        previous.as_ref().is_some_and(|p| p.year == v.year),
        v.year.map(|y| y.to_string()),
    );
    update(
        "doi",
        previous.as_ref().is_some_and(|p| p.doi == v.doi),
        (!v.doi.is_empty()).then(|| escape(&v.doi)),
    );
    update(
        "url",
        previous.as_ref().is_some_and(|p| p.url == v.url),
        (!v.url.is_empty()).then(|| escape(&v.url)),
    );
    let venue_key = if fields.contains_key("journal") {
        "journal"
    } else if fields.contains_key("booktitle") || kind == "inproceedings" {
        "booktitle"
    } else {
        "journal"
    };
    if !previous
        .as_ref()
        .is_some_and(|p| p.journal_or_conference == v.journal_or_conference)
    {
        if v.journal_or_conference.is_empty() {
            fields.remove(venue_key);
        } else {
            fields.insert(venue_key.into(), escape(&v.journal_or_conference));
        }
    }
    Ok(format!(
        "@{kind}{{{},\n{}\n}}",
        v.bibtex_key,
        fields
            .iter()
            .map(|(k, v)| format!("  {k} = {{{v}}},"))
            .collect::<Vec<_>>()
            .join("\n")
    ))
}
pub fn crossref(input: &Value, doi: &str) -> Result<ReferenceInput> {
    let m = input
        .get("message")
        .filter(|m| m.is_object())
        .ok_or("Crossref 响应缺少 message。")?;
    let first = |k: &str| {
        m.get(k)
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    };
    let mut names = vec![];
    if let Some(authors) = m.get("author") {
        let a = authors.as_array().ok_or("Crossref 作者结构无效。")?;
        for name in a {
            if !name.is_object()
                || ["name", "given", "family"]
                    .iter()
                    .any(|key| name.get(key).is_some_and(|v| !v.is_string()))
            {
                return Err("Crossref 作者字段格式无效，请手工填写。".into());
            }
            let corporate = name.get("name").and_then(Value::as_str);
            let given = name.get("given").and_then(Value::as_str).unwrap_or("");
            let family = name.get("family").and_then(Value::as_str).unwrap_or("");
            let full = corporate
                .map(str::to_owned)
                .unwrap_or_else(|| format!("{given} {family}").trim().to_owned());
            if !full.is_empty() {
                names.push(full);
            }
        }
    }
    let mut year = None;
    for key in ["published", "published-print", "published-online", "issued"] {
        if let Some(parts) = m.get(key).and_then(|v| v.get("date-parts")) {
            let value = parts
                .as_array()
                .and_then(|a| a.first())
                .and_then(Value::as_array)
                .and_then(|a| a.first())
                .and_then(Value::as_i64)
                .ok_or("Crossref 年份结构无效，请手工填写。")?;
            if !(1..=9999).contains(&value) {
                return Err("Crossref 年份超出范围。".into());
            }
            year = Some(i32::try_from(value).map_err(|_| "Crossref 年份超出范围。")?);
            break;
        }
    }
    let received = normalize_doi(m.get("DOI").and_then(Value::as_str).unwrap_or(doi))?;
    if received != normalize_doi(doi)? {
        return Err("Crossref 返回的 DOI 与查询不一致。".into());
    }
    let item = ReferenceInput {
        title: first("title"),
        authors: names,
        year,
        journal_or_conference: first("container-title"),
        doi: received,
        url: m.get("URL").and_then(Value::as_str).unwrap_or("").into(),
        bibtex_key: String::new(),
        bibtex_entry: String::new(),
        verified_source: "crossref".into(),
        source_metadata: serde_json::json!({"provider":"Crossref","doi":doi,"retrievedMetadata":m}),
    };
    // Empty title is shown as missing in preview; saving requires user input. No invented fields.
    if item.source_metadata.to_string().len() > 262144 {
        return Err("Crossref 元数据过大。".into());
    }
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn doi_normalization_and_rejection() {
        assert_eq!(
            normalize_doi("https://doi.org/10.1234/AbC").unwrap(),
            "10.1234/abc"
        );
        for v in ["10./a", "10.123/a", "10.1234/", "10.1234/a b"] {
            assert!(normalize_doi(v).is_err());
        }
    }
    #[test]
    fn bib_multi_unicode_braces_quotes_and_raw_preservation() {
        let s = "@article{Li2026, title={中文 {LaTeX} $x^2$}, author={{研究院 and 团队} and Li, Wang},year=2026,journal=\"Research\",note={retained}}\n@misc{B,title={Second}}";
        let items = import_bib(s).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].authors.len(), 2);
        assert_eq!(bibtex(&items[0]).unwrap(), items[0].bibtex_entry);
        let mut edit = items[0].clone();
        edit.title = "Edited".into();
        assert!(bibtex(&edit).unwrap().contains("note = {retained}"));
    }
    #[test]
    fn editing_a_different_field_preserves_imported_math_and_corporate_authors() {
        let mut item=import_bib(r"@article{Math,title={{An} $x_1$ \alpha},author={{Institute and Team} and Li, Ming},year=2026,note={unchanged}}").unwrap().remove(0);
        item.journal_or_conference = "New Journal".into();
        let result = bibtex(&item).unwrap();
        assert!(result.contains(r"title = {{An} $x_1$ \alpha}"));
        assert!(result.contains("author = {{Institute and Team} and Li, Ming}"));
        assert!(result.contains("note = {unchanged}"));
        assert!(
            crossref(
                &serde_json::json!({"message":{"author":[{"given":123}]}}),
                "10.1234/a"
            )
            .is_err()
        );
        assert!(
            crossref(
                &serde_json::json!({"message":{"issued":{"date-parts":[[0]]}}}),
                "10.1234/a"
            )
            .is_err()
        );
    }
    #[test]
    fn rejects_bad_year_macros_overflow_and_invalid_key() {
        for s in [
            "@article{key,title={a},year={oops}}",
            "@misc{a,title=macro}",
            "@misc{a,title={a} # {b}}",
            "@misc{a.b,title={x}}",
            "@misc{a,title={unclosed}",
            "@string{a={b}}",
        ] {
            assert!(import_bib(s).is_err(), "{s}");
        }
        assert!(import_bib(&"x".repeat(MAX_BIB_BYTES + 1)).is_err());
    }
    #[test]
    fn crossref_mapping_missing_fields_and_bad_year() {
        let v = serde_json::json!({"message":{"title":["A"],"author":[{"given":"A","family":"Li"},{"name":"研究院"}],"published":{"date-parts":[[2025,1]]},"DOI":"10.1234/A"}});
        let result = crossref(&v, "10.1234/a").unwrap();
        assert_eq!(result.authors, vec!["A Li", "研究院"]);
        assert_eq!(result.year, Some(2025));
        assert_eq!(result.journal_or_conference, "");
        assert!(
            crossref(
                &serde_json::json!({"message":{"published":{"date-parts":[["oops"]]}}}),
                "10.1234/a"
            )
            .is_err()
        );
        let blank = crossref(&serde_json::json!({"message":{}}), "10.1234/a").unwrap();
        assert_eq!(blank.year, None);
        assert!(blank.title.is_empty());
    }
}
