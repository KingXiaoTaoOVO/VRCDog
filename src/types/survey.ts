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
}

export interface SurveyReward {
  /** Role granted to a user after a passing submission. */
  role_id: string;
  /** Hours the granted role stays active; null/undefined means permanent. */
  duration_hours: number | null;
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
}
