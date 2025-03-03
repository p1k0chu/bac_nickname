use merge_json::Merge;
use regex::{Captures, Regex};
use serde_json::Value;
use std::error::Error;
use std::format;
use std::fs::{self, File};
use std::path::Path;

/// Parses all json files from the folder specified by `path` and merges them
pub fn parse_and_merge(path: &Path) -> Result<Value, Box<dyn Error>> {
    let mut result = Value::Null;

    for file in fs::read_dir(path)? {
        let file = file?.path();

        if file.extension().is_none_or(|x| x != "json") {
            continue;
        }

        let reader = File::open(file)?;
        let j: Value = serde_json::from_reader(&reader)?;

        result.merge(&j);
    }

    Ok(result)
}

/// parses `string` and gets advancement ids from it, then replaces them with progresses from json
///
/// # Examples
/// ```rust
/// use serde_json::json;
/// use bac_nickname::replace_with_progress;
///
/// let j = json!({
///     "advancement_id": {
///         "criteria": {
///             "one": 0,
///             "two": 0
///         }
///     }
/// });
/// 
/// let input = "prefix (advancement_id) suffix";
/// let output: String = String::from("prefix 2 suffix");
///
/// assert_eq!(&replace_with_progress(input, &j), &output);
/// ```
pub fn replace_with_progress(string: &str, json: &Value) -> String {
    let re = Regex::new(r"(\(.+\))").unwrap();

    re.replace_all(string, |caps: &Captures| {
        let s = &caps[0][1..caps[0].len() - 1];

        match get_progress(json, s) {
            Ok(x) => x.to_string(),
            Err(_) => caps[0].to_string(),
        }
    })
    .to_string()
}

/// counts progress for specific advancement (by id) in the json
///
/// # Examples
/// ```rust
/// use serde_json::json;
/// use bac_nickname::get_progress;
///
/// let j = json!({
///     "advancement_id": {
///         "criteria": {
///             "one": 0,
///             "two": 0
///         }
///     }
/// });
///
/// assert!(get_progress(&j, "advancement_id").is_ok_and(|x| x == 2usize));
/// assert!(get_progress(&j, "blah").is_err());
/// ```
pub fn get_progress(json: &Value, adv_id: &str) -> Result<usize, Box<dyn Error>> {
    let json = json.get(adv_id).ok_or(Box::<dyn Error>::from(format!(
        "couldn't find advancement {}",
        adv_id
    )))?;

    let criteria =
        json.get("criteria")
            .and_then(Value::as_object)
            .ok_or(Box::<dyn Error>::from(format!(
                "key 'criteria' not found for advancement {}",
                adv_id
            )))?;

    Ok(criteria.iter().count())
}

#[cfg(test)]
mod tests {
    use std::fs::OpenOptions;

    use super::*;
    use serde_json::json;
    use tempdir::TempDir;

    fn test_json() -> Value {
        json!({
            "advancement": {
                "criteria": {
                    "one": 0,
                    "two": 0
                }
            }
        })
    }

    #[test]
    fn get_progress_test() {
        let j = test_json();

        assert!(get_progress(&j, "advancement").is_ok_and(|x| x == 2usize));
        assert!(get_progress(&j, "blah").is_err());
    }

    #[test]
    fn replace_test() {
        let input: &str = "Name (advancement)/69";
        let output = String::from("Name 2/69");

        assert_eq!(&replace_with_progress(input, &test_json()), &output);
    }

    #[test]
    fn parse_and_merge_test() -> Result<(), Box<dyn Error>> {
        // name doesn't matter
        let dir = TempDir::new("advancements")?;
        let path = dir.path();

        let j = json!({"advancement": {"criteria": {"first": 0}}});
        serde_json::to_writer(
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(path.join("file1.json"))?,
            &j,
        )?;

        let j = json!({"advancement": {"criteria": {"second": 0}}});
        serde_json::to_writer(
            OpenOptions::new()
                .write(true)
                .create(true)
                .open(path.join("file2.json"))?,
            &j,
        )?;

        fs::write(path.join("non-json-file.txt"), "Hello world!")?;

        let j = json!({"advancement": {"criteria": {"first":0, "second": 0}}});

        assert_eq!(&(parse_and_merge(path)?), &j);

        Ok(())
    }
}
