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

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct SurveySettings {
    #[serde(default)]
    pub enabled: bool,
}

impl Default for SurveySettings {
    fn default() -> Self {
        Self { enabled: false }
    }
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

#[derive(Debug, PartialEq, Eq)]
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
    survey.description = survey.description.trim().to_string();
    if survey.title.is_empty() {
        return Err("Survey title is required".to_string());
    }
    if survey.questions.is_empty() {
        return Err("At least one question is required".to_string());
    }
    if survey.survey_id.trim().is_empty() {
        survey.survey_id = new_id("survey");
    }
    if survey.revision == 0 {
        survey.revision = 1;
    }
    if !matches!(survey.status.as_str(), "draft" | "published") {
        survey.status = "draft".to_string();
    }

    let mut question_ids = HashSet::new();
    for question in &mut survey.questions {
        question.title = question.title.trim().to_string();
        if question.title.is_empty() {
            return Err("Every question needs a title".to_string());
        }
        if question.question_id.trim().is_empty() {
            question.question_id = new_id("question");
        }
        if !question_ids.insert(question.question_id.clone()) {
            return Err("Question IDs must be unique".to_string());
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
            return Err("Choice questions need at least two options".to_string());
        }
        let mut option_ids = HashSet::new();
        for option in &mut question.options {
            option.label = option.label.trim().to_string();
            if option.option_id.trim().is_empty() {
                option.option_id = new_id("option");
            }
            if option.label.is_empty() || !option_ids.insert(option.option_id.clone()) {
                return Err("Options need unique IDs and non-empty labels".to_string());
            }
        }
        if is_choice
            && question
                .correct_answers
                .iter()
                .any(|answer| !option_ids.contains(answer))
        {
            return Err("A correct answer references a missing option".to_string());
        }
        if question.require_correct && question.correct_answers.is_empty() {
            return Err("Questions that gate access need at least one correct answer".to_string());
        }
        for media in &mut question.media {
            media.url = media.url.trim().to_string();
            if !matches!(media.media_type.as_str(), "image" | "video") {
                return Err("Media type must be image or video".to_string());
            }
            if !(media.url.starts_with("https://") || media.url.starts_with("http://")) {
                return Err("Media URL must start with http:// or https://".to_string());
            }
        }
    }
    Ok(())
}

fn answer_values(value: Option<&Value>) -> Vec<String> {
    match value {
        Some(Value::String(text)) => {
            let normalized = text.trim();
            if normalized.is_empty() {
                Vec::new()
            } else {
                vec![normalized.to_string()]
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_survey() -> Survey {
        Survey {
            survey_id: "survey_1".into(),
            title: "Access check".into(),
            description: String::new(),
            required_for_access: true,
            status: "published".into(),
            revision: 1,
            created_at: String::new(),
            updated_at: String::new(),
            published_at: None,
            questions: vec![SurveyQuestion {
                question_id: "q1".into(),
                question_type: "multiple_choice".into(),
                title: "Choose both".into(),
                description: String::new(),
                required: true,
                require_correct: true,
                options: vec![
                    SurveyOption {
                        option_id: "a".into(),
                        label: "A".into(),
                    },
                    SurveyOption {
                        option_id: "b".into(),
                        label: "B".into(),
                    },
                ],
                correct_answers: vec!["a".into(), "b".into()],
                media: Vec::new(),
            }],
        }
    }

    #[test]
    fn exact_multiple_choice_answer_passes() {
        let answers = HashMap::from([("q1".into(), json!(["b", "a"]))]);
        assert!(evaluate(&sample_survey(), &answers).passed);
    }

    #[test]
    fn partial_multiple_choice_answer_fails() {
        let answers = HashMap::from([("q1".into(), json!(["a"]))]);
        assert_eq!(
            evaluate(&sample_survey(), &answers).failed_question_ids,
            vec!["q1"]
        );
    }

    #[test]
    fn public_copy_never_exposes_correct_answers() {
        assert!(sample_survey().public_copy().questions[0]
            .correct_answers
            .is_empty());
    }

    #[test]
    fn passed_revision_is_not_pending_but_new_revision_is() {
        let survey = sample_survey();
        let surveys = HashMap::from([(survey.survey_id.clone(), survey.clone())]);
        let submissions = HashMap::from([(
            "s1".into(),
            SurveySubmission {
                submission_id: "s1".into(),
                survey_id: survey.survey_id.clone(),
                survey_revision: 1,
                survey_title: survey.title.clone(),
                user_id: "user".into(),
                submitted_at: String::new(),
                status: "passed".into(),
                passed: true,
                answers: HashMap::new(),
                failed_question_ids: Vec::new(),
            },
        )]);
        assert!(pending_surveys(true, &surveys, &submissions, "user").is_empty());

        let mut revised = survey;
        revised.revision = 2;
        let revised_surveys = HashMap::from([(revised.survey_id.clone(), revised)]);
        assert_eq!(
            pending_surveys(true, &revised_surveys, &submissions, "user").len(),
            1
        );
    }
}
