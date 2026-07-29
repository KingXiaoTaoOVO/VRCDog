import type { Survey, SurveyQuestion, SurveySubmission } from '../types/survey';

export interface SurveyExportUser {
  user_id: string;
  display_name: string;
}

export interface SurveyExportTable {
  headers: string[];
  rows: string[][];
  widths: number[];
}

const EXCEL_FORMULA_PREFIX = /^[=+\-@]/;

export const safeExcelText = (value: unknown): string => {
  const text = value === undefined || value === null ? '' : String(value);
  return EXCEL_FORMULA_PREFIX.test(text.trimStart()) ? `'${text}` : text;
};

const answerText = (
  answer: string | string[] | undefined,
  question?: SurveyQuestion,
): string => {
  if (answer === undefined || answer === null) return '';
  const values = Array.isArray(answer) ? answer : [answer];
  return values
    .map((value) => {
      const text = String(value).trim();
      if (!text) return '';
      return question?.options.find((option) => option.option_id === text)?.label || text;
    })
    .filter(Boolean)
    .join('、');
};

const submissionStatus = (submission: SurveySubmission): string => {
  if (submission.status === 'dismissed') return '已忽略';
  return submission.passed ? '通过' : '未通过';
};

const questionHeader = (question: SurveyQuestion, index: number): string => (
  `Q${index + 1}：${question.title.trim() || '未命名题目'}`
);

export const buildSurveyExportTable = (
  survey: Survey,
  submissions: SurveySubmission[],
  users: SurveyExportUser[],
): SurveyExportTable => {
  const userMap = new Map(users.map((user) => [user.user_id, user.display_name.trim()]));
  const currentQuestionMap = new Map(survey.questions.map((question) => [question.question_id, question]));
  const historicalQuestionIds: string[] = [];

  for (const submission of submissions) {
    for (const questionId of Object.keys(submission.answers)) {
      if (!currentQuestionMap.has(questionId) && !historicalQuestionIds.includes(questionId)) {
        historicalQuestionIds.push(questionId);
      }
    }
  }

  const fixedHeaders = [
    '用户名',
    'VRChat 用户 ID',
    '提交时间',
    '结果',
    '问卷版本',
    '错误题数',
    '提交 ID',
  ];
  const questionHeaders = survey.questions.map(questionHeader);
  const historicalHeaders = historicalQuestionIds.map((questionId, index) => (
    `历史题目 ${index + 1}（${questionId}）`
  ));

  const rows = submissions.map((submission) => {
    const displayName = userMap.get(submission.user_id) || submission.user_id;
    const fixedValues = [
      displayName,
      submission.user_id,
      submission.submitted_at,
      submissionStatus(submission),
      `v${submission.survey_revision}`,
      String(submission.failed_question_ids.length),
      submission.submission_id,
    ];
    const questionValues = survey.questions.map((question) => (
      answerText(submission.answers[question.question_id], question)
    ));
    const historicalValues = historicalQuestionIds.map((questionId) => (
      answerText(submission.answers[questionId])
    ));
    return [...fixedValues, ...questionValues, ...historicalValues].map(safeExcelText);
  });

  return {
    headers: [...fixedHeaders, ...questionHeaders, ...historicalHeaders],
    rows,
    widths: [22, 42, 22, 12, 12, 12, 38, ...questionHeaders.map(() => 32), ...historicalHeaders.map(() => 32)],
  };
};

export const surveyExportFileName = (surveyTitle: string, date = new Date()): string => {
  const safeTitle = surveyTitle
    .trim()
    .replace(/[<>:"/\\|?*\u0000-\u001F]/g, '_')
    .replace(/[. ]+$/g, '')
    .slice(0, 80) || '未命名问卷';
  const dateText = [
    date.getFullYear(),
    String(date.getMonth() + 1).padStart(2, '0'),
    String(date.getDate()).padStart(2, '0'),
  ].join('');
  return `${safeTitle}_答卷记录_${dateText}.xlsx`;
};

export const buildSurveyWorkbook = async (
  survey: Survey,
  submissions: SurveySubmission[],
  users: SurveyExportUser[],
): Promise<Uint8Array> => {
  const ExcelJS = await import('exceljs');
  const workbook = new ExcelJS.Workbook();
  workbook.creator = 'VrcDog';
  workbook.created = new Date();

  const sheetName = (survey.title.trim() || '答卷记录')
    .replace(/[\\/*?:[\]]/g, '_')
    .slice(0, 31);
  const worksheet = workbook.addWorksheet(sheetName || '答卷记录', {
    views: [{ state: 'frozen', ySplit: 1 }],
  });
  const table = buildSurveyExportTable(survey, submissions, users);

  worksheet.addRow(table.headers);
  for (const row of table.rows) worksheet.addRow(row);
  worksheet.columns.forEach((column, index) => {
    column.width = table.widths[index] || 24;
    column.alignment = { vertical: 'top', wrapText: true };
  });

  const header = worksheet.getRow(1);
  header.height = 24;
  header.font = { bold: true, color: { argb: 'FFFFFFFF' } };
  header.fill = { type: 'pattern', pattern: 'solid', fgColor: { argb: 'FFE77900' } };
  header.alignment = { vertical: 'middle', horizontal: 'center', wrapText: true };

  if (table.headers.length > 0) {
    worksheet.autoFilter = {
      from: { row: 1, column: 1 },
      to: { row: 1, column: table.headers.length },
    };
  }
  worksheet.eachRow((row, rowNumber) => {
    if (rowNumber > 1) {
      row.alignment = { vertical: 'top', wrapText: true };
      if (rowNumber % 2 === 1) {
        row.fill = { type: 'pattern', pattern: 'solid', fgColor: { argb: 'FFFFF8EE' } };
      }
    }
  });

  const buffer = await workbook.xlsx.writeBuffer();
  return new Uint8Array(buffer);
};
