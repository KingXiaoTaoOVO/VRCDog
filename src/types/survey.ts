export type SurveyQuestionType = 'single_choice' | 'multiple_choice' | 'short_text' | 'long_text';
export type SurveyMediaType = 'image' | 'video';
export type SurveyStatus = 'draft' | 'published';

export interface SurveyMedia {
  media_type: SurveyMediaType;
  url: string;
  caption: string;
}

export interface SurveyOption {
  option_id: string;
  label: string;
  /** Optional images/videos attached to this choice option. */
  media?: SurveyMedia[];
}

export interface SurveyReward {
  /** Role granted to a user after a passing submission. */
  role_id: string;
  /** How long the granted role stays active, in `duration_unit` units; null/undefined means permanent. */
  duration_value: number | null;
  /** Granularity of `duration_value`: 'hour' | 'day' | 'month' | 'year'. */
  duration_unit: string;
}

export interface SurveyQuestion {
  question_id: string;
  question_type: SurveyQuestionType;
  title: string;
  description: string;
  required: boolean;
  require_correct: boolean;
  options: SurveyOption[];
  correct_answers: string[];
  media: SurveyMedia[];
}

export interface Survey {
  survey_id: string;
  title: string;
  description: string;
  required_for_access: boolean;
  status: SurveyStatus;
  revision: number;
  created_at: string;
  updated_at: string;
  published_at: string | null;
  questions: SurveyQuestion[];
  /** Optional incentive granted (role, temporary or permanent) on a passing submission. */
  reward: SurveyReward | null;
}

export interface SurveyAnswerAttachment {
  file_id: string;
  file_name: string;
  mime_type: string;
  size: number;
  url: string;
}

export interface SurveyClickEvent {
  event_id: string;
  survey_id: string;
  survey_revision: number;
  survey_title: string;
  user_id: string;
  question_id: string;
  question_title: string;
  option_id: string;
  option_label: string;
  action: 'select' | 'deselect' | 'input';
  text_value: string;
  clicked_at: string;
  submission_id: string;
}

export interface SurveySubmission {
  submission_id: string;
  survey_id: string;
  survey_revision: number;
  survey_title: string;
  user_id: string;
  submitted_at: string;
  status: 'passed' | 'failed' | 'dismissed';
  passed: boolean;
  answers: Record<string, string | string[]>;
  failed_question_ids: string[];
  /** Per-question file attachments uploaded by the respondent. */
  answer_files: Record<string, SurveyAnswerAttachment[]>;
  /** Option-level click events recorded while the respondent filled the survey. */
  click_events?: SurveyClickEvent[];
}
