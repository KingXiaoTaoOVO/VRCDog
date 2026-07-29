use chrono::Local;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::{AtomicU64, Ordering},
};

static ID_SEQUENCE: AtomicU64 = AtomicU64::new(1);

pub fn new_id(prefix: &str) -> String {
    format!(
        "{prefix}_{}_{}",
        Local::now().timestamp_millis(),
        ID_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    )
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
pub struct SurveySettings {
    #[serde(default)]
    pub enabled: bool,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SurveyMedia {
    pub media_type: String,
    pub url: String,
    #[serde(default)]
    pub caption: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SurveyOption {
    pub option_id: String,
    pub label: String,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SurveyQuestion {
    pub question_id: String,
    pub question_type: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub require_correct: bool,
    #[serde(default)]
    pub options: Vec<SurveyOption>,
    #[serde(default)]
    pub correct_answers: Vec<String>,
    #[serde(default)]
    pub media: Vec<SurveyMedia>,
}

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Survey {
    pub survey_id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub required_for_access: bool,
    #[serde(default = "default_status")]
    pub status: String,
    #[serde(default = "default_revision")]
    pub revision: u32,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub published_at: Option<String>,
    #[serde(default)]
    pub questions: Vec<SurveyQuestion>,
}

fn default_status() -> String {
    "draft".to_string()
}
fn default_revision() -> u32 {
    1
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SurveySubmission {
    pub submission_id: String,
    pub survey_id: String,
    pub survey_revision: u32,
    pub survey_title: String,
    pub user_id: String,
    pub submitted_at: String,
    pub status: String,
    pub passed: bool,
    #[serde(default)]
    pub answers: HashMap<String, Value>,
    #[serde(default)]
    pub failed_question_ids: Vec<String>,
}

pub struct Evaluation {
    pub passed: bool,
    pub failed_question_ids: Vec<String>,
}

impl Survey {
    pub fn public_copy(&self) -> Self {
        let mut survey = self.clone();
        for question in &mut survey.questions {
            question.correct_answers.clear();
        }
        survey
    }
}

pub fn validate_survey(survey: &mut Survey) -> Result<(), String> {
    survey.title = survey.title.trim().to_string();
    if survey.title.is_empty() {
        return Err("Survey title is required".into());
    }
    if survey.questions.is_empty() {
        return Err("At least one question is required".into());
    }
    if survey.survey_id.trim().is_empty() {
        survey.survey_id = new_id("survey");
    }
    if survey.revision == 0 {
        survey.revision = 1;
    }
    if !matches!(survey.status.as_str(), "draft" | "published") {
        survey.status = "draft".into();
    }
    let mut question_ids = HashSet::new();
    for question in &mut survey.questions {
        question.title = question.title.trim().to_string();
        if question.title.is_empty() {
            return Err("Every question needs a title".into());
        }
        if question.question_id.trim().is_empty() {
            question.question_id = new_id("question");
        }
        if !question_ids.insert(question.question_id.clone()) {
            return Err("Question IDs must be unique".into());
        }
        if !matches!(
            question.question_type.as_str(),
            "single_choice" | "multiple_choice" | "short_text" | "long_text"
        ) {
            return Err(format!(
                "Unsupported question type: {}",
                question.question_type
            ));
        }
        let is_choice = matches!(
            question.question_type.as_str(),
            "single_choice" | "multiple_choice"
        );
        if is_choice && question.options.len() < 2 {
            return Err("Choice questions need at least two options".into());
        }
        let mut option_ids = HashSet::new();
        for option in &mut question.options {
            option.label = option.label.trim().to_string();
            if option.option_id.trim().is_empty() {
                option.option_id = new_id("option");
            }
            if option.label.is_empty() || !option_ids.insert(option.option_id.clone()) {
                return Err("Options need unique IDs and non-empty labels".into());
            }
        }
        if is_choice
            && question
                .correct_answers
                .iter()
                .any(|answer| !option_ids.contains(answer))
        {
            return Err("A correct answer references a missing option".into());
        }
        if question.require_correct && question.correct_answers.is_empty() {
            return Err("Questions that gate access need at least one correct answer".into());
        }
        for media in &mut question.media {
            media.url = media.url.trim().to_string();
            if !matches!(media.media_type.as_str(), "image" | "video") {
                return Err("Media type must be image or video".into());
            }
            if !(media.url.starts_with("https://") || media.url.starts_with("http://")) {
                return Err("Media URL must start with http:// or https://".into());
            }
        }
    }
    Ok(())
}

fn answer_values(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) if !text.trim().is_empty() => vec![text.trim().to_string()],
        Some(Value::Array(values)) => values
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

pub fn evaluate(survey: &Survey, answers: &HashMap<String, Value>) -> Evaluation {
    let mut failed = Vec::new();
    for question in &survey.questions {
        let actual = answer_values(answers.get(&question.question_id));
        if question.required && actual.is_empty() {
            failed.push(question.question_id.clone());
            continue;
        }
        if !question.require_correct || actual.is_empty() {
            continue;
        }
        let correct = if matches!(question.question_type.as_str(), "short_text" | "long_text") {
            let accepted: HashSet<String> = question
                .correct_answers
                .iter()
                .map(|answer| answer.trim().to_lowercase())
                .collect();
            actual
                .first()
                .is_some_and(|answer| accepted.contains(&answer.to_lowercase()))
        } else {
            let expected: HashSet<&str> = question
                .correct_answers
                .iter()
                .map(String::as_str)
                .collect();
            let received: HashSet<&str> = actual.iter().map(String::as_str).collect();
            expected == received
        };
        if !correct {
            failed.push(question.question_id.clone());
        }
    }
    Evaluation {
        passed: failed.is_empty(),
        failed_question_ids: failed,
    }
}

pub fn pending_surveys(
    enabled: bool,
    surveys: &HashMap<String, Survey>,
    submissions: &HashMap<String, SurveySubmission>,
    user_id: &str,
) -> Vec<Survey> {
    if !enabled {
        return Vec::new();
    }
    let mut pending: Vec<Survey> = surveys
        .values()
        .filter(|survey| survey.status == "published")
        .filter(|survey| {
            !submissions.values().any(|submission| {
                submission.user_id == user_id
                    && submission.survey_id == survey.survey_id
                    && submission.survey_revision == survey.revision
                    && matches!(submission.status.as_str(), "passed" | "dismissed")
            })
        })
        .map(Survey::public_copy)
        .collect();
    pending.sort_by(|left, right| left.published_at.cmp(&right.published_at));
    pending
}
