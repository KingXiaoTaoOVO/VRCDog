import { describe, expect, it } from 'vitest';
import type { Survey, SurveySubmission } from '../types/survey';
import {
  buildSurveyExportTable,
  safeExcelText,
  surveyExportFileName,
} from './surveyExcel';

const survey: Survey = {
  survey_id: 'survey_1',
  title: '入群/测试:*?',
  description: '',
  required_for_access: true,
  status: 'published',
  revision: 2,
  created_at: '',
  updated_at: '',
  published_at: '',
  reward: null,
  questions: [
    {
      question_id: 'q_choice',
      question_type: 'single_choice',
      title: '第一眼看见什么？',
      description: '',
      required: true,
      require_correct: true,
      options: [
        { option_id: 'option_x', label: '小狗' },
        { option_id: 'option_y', label: '小猫' },
      ],
      correct_answers: ['option_x'],
      media: [],
    },
    {
      question_id: 'q_multi',
      question_type: 'multiple_choice',
      title: '喜欢哪些颜色？',
      description: '',
      required: false,
      require_correct: false,
      options: [
        { option_id: 'red', label: '红色' },
        { option_id: 'blue', label: '蓝色' },
      ],
      correct_answers: [],
      media: [],
    },
    {
      question_id: 'q_text',
      question_type: 'short_text',
      title: '备注',
      description: '',
      required: false,
      require_correct: false,
      options: [],
      correct_answers: [],
      media: [],
    },
  ],
};

const submission: SurveySubmission = {
  submission_id: 'submission_1',
  survey_id: 'survey_1',
  survey_revision: 1,
  survey_title: '入群测试',
  user_id: 'usr_123',
  submitted_at: '2026-07-29 19:00:00',
  status: 'failed',
  passed: false,
  answers: {
    q_choice: 'option_x',
    q_multi: ['red', 'blue'],
    q_text: '=HYPERLINK("bad")',
    old_question: '旧答案',
  },
  failed_question_ids: ['q_text'],
  answer_files: {},
};

describe('survey Excel export table', () => {
  it('creates one row per submission with username, dynamic questions and labels', () => {
    const table = buildSurveyExportTable(
      survey,
      [submission],
      [{ user_id: 'usr_123', display_name: '小明' }],
    );

    expect(table.rows).toHaveLength(1);
    expect(table.headers).toContain('Q1：第一眼看见什么？');
    expect(table.headers.some((header) => header.includes('old_question'))).toBe(true);
    expect(table.rows[0].slice(0, 7)).toEqual([
      '小明',
      'usr_123',
      '2026-07-29 19:00:00',
      '未通过',
      'v1',
      '1',
      'submission_1',
    ]);
    expect(table.rows[0]).toContain('小狗');
    expect(table.rows[0]).toContain('红色、蓝色');
    expect(table.rows[0]).toContain('旧答案');
  });

  it('falls back to user ID and neutralizes formula-like text', () => {
    const table = buildSurveyExportTable(survey, [submission], []);
    expect(table.rows[0][0]).toBe('usr_123');
    expect(table.rows[0]).toContain("'=HYPERLINK(\"bad\")");
    expect(safeExcelText(' +SUM(1,2)')).toBe("' +SUM(1,2)");
  });

  it('sanitizes the Windows filename and includes the date', () => {
    expect(surveyExportFileName(survey.title, new Date(2026, 6, 29)))
      .toBe('入群_测试____答卷记录_20260729.xlsx');
  });
});
